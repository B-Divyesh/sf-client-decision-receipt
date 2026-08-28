use crate::{
    db,
    error::AppError,
    mail::Mailer,
    model::{self, CreateProposal, CreatedProposal, ManagedProposal, Proposal, SubmitDecision},
};
use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Path, State},
    http::{header, HeaderName, HeaderValue, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use rand::{rngs::OsRng, RngCore};
use serde::Deserialize;
use serde_json::json;
use sha2::Sha256;
use sqlx::SqlitePool;
use std::{
    net::SocketAddr,
    path::{Path as FilePath, PathBuf},
    sync::Arc,
};
use tokio::{fs, net::TcpListener};
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorLayer,
};
use tower_http::{
    compression::CompressionLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

#[derive(Clone)]
pub struct AppState {
    pool: SqlitePool,
    secret: Arc<Vec<u8>>,
    mailer: Option<Mailer>,
}

pub async fn run() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8080);
    let data_dir = PathBuf::from(std::env::var("DATA_DIR").unwrap_or_else(|_| "/data".into()));
    fs::create_dir_all(&data_dir).await?;
    let (secret, secret_source) = load_secret(&data_dir).await?;
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| format!("sqlite://{}", data_dir.join("receipts.sqlite").display()));
    let pool = db::connect(&db_url).await?;
    let mailer = Mailer::from_env()?;
    let mail_source = if mailer.is_some() {
        "supplied SMTP"
    } else {
        "durable queue"
    };
    let state = AppState {
        pool,
        secret: Arc::new(secret),
        mailer,
    };
    if let Some(mailer) = state.mailer.clone() {
        let pending_pool = state.pool.clone();
        tokio::spawn(async move {
            mailer.flush_pending(&pending_pool).await;
        });
    }
    let dist = std::env::var("DIST_DIR").unwrap_or_else(|_| "dist".into());
    let app = router(state, &dist);
    let address = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!(
        port,
        build_sha = env!("BUILD_SHA"),
        instance_secret = secret_source,
        mail_delivery = mail_source,
        "service starting"
    );
    let listener = TcpListener::bind(address).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown())
    .await?;
    Ok(())
}

pub fn router(state: AppState, dist: &str) -> Router {
    let mut read_builder = GovernorConfigBuilder::default();
    let read_config = read_builder
        .per_millisecond(50)
        .burst_size(40)
        .key_extractor(SmartIpKeyExtractor)
        .finish()
        .unwrap();
    let mut write_builder = GovernorConfigBuilder::default();
    let write_config = write_builder
        .per_second(1)
        .burst_size(6)
        .key_extractor(SmartIpKeyExtractor)
        .finish()
        .unwrap();

    let reads = Router::new()
        .route("/proposals/{token}", get(get_proposal))
        .route("/manage/{token}", get(get_managed))
        .route("/manage/{token}/export.json", get(export_json))
        .route("/manage/{token}/export.csv", get(export_csv));
    let writes = Router::new()
        .route("/proposals", post(create_proposal))
        .route("/proposals/{token}/decision", post(submit_decision))
        .route("/manage/{token}", delete(delete_proposal))
        .layer(GovernorLayer::new(write_config));
    let api = Router::new()
        .merge(reads)
        .merge(writes)
        .fallback(api_not_found)
        .layer(GovernorLayer::new(read_config))
        .with_state(state);
    let fallback = ServeDir::new(dist)
        .append_index_html_on_directories(true)
        .fallback(ServeFile::new(format!("{dist}/index.html")));
    Router::new()
        .route("/health", get(health))
        .nest("/api", api)
        .fallback_service(fallback)
        .layer(DefaultBodyLimit::max(256 * 1024))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(security_headers))
}

async fn health() -> impl IntoResponse {
    Json(json!({ "status": "ok", "buildSha": env!("BUILD_SHA") }))
}

async fn api_not_found() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(json!({ "error": "API route not found." })),
    )
}

async fn create_proposal(
    State(state): State<AppState>,
    Json(input): Json<CreateProposal>,
) -> Result<(StatusCode, Json<CreatedProposal>), AppError> {
    model::validate_create(&input).map_err(|e| AppError::BadRequest(e.into()))?;
    let client_token = token();
    let manage_token = token();
    let id = format!(
        "CDR-{}-{}",
        chrono::Utc::now().format("%Y"),
        token()[..8].to_ascii_uppercase()
    );
    db::insert_proposal(
        &state.pool,
        &id,
        &client_token,
        &token_hash(&state.secret, &client_token),
        &token_hash(&state.secret, &manage_token),
        &input,
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(CreatedProposal {
            id,
            client_path: format!("/p/{client_token}"),
            manage_path: format!("/m/{manage_token}"),
        }),
    ))
}

async fn get_proposal(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<Proposal>, AppError> {
    valid_token(&token)?;
    Ok(Json(
        db::by_client(&state.pool, &token_hash(&state.secret, &token)).await?,
    ))
}

async fn submit_decision(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Json(input): Json<SubmitDecision>,
) -> Result<(StatusCode, Json<Proposal>), AppError> {
    valid_token(&token)?;
    model::validate_decision(&input).map_err(|e| AppError::BadRequest(e.into()))?;
    let proposal = db::decide(&state.pool, &token_hash(&state.secret, &token), &input).await?;
    if let Some(mailer) = state.mailer.clone() {
        let pool = state.pool.clone();
        let id = proposal.id.clone();
        tokio::spawn(async move {
            mailer.flush_proposal(&pool, &id).await;
        });
    }
    Ok((StatusCode::CREATED, Json(proposal)))
}

async fn get_managed(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<ManagedProposal>, AppError> {
    valid_token(&token)?;
    Ok(Json(
        db::by_manage(&state.pool, &token_hash(&state.secret, &token)).await?,
    ))
}

async fn export_json(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Response, AppError> {
    valid_token(&token)?;
    let proposal = db::by_manage(&state.pool, &token_hash(&state.secret, &token)).await?;
    let body = serde_json::to_vec_pretty(&proposal).map_err(anyhow::Error::from)?;
    Ok(download(
        body,
        "application/json",
        &format!("{}-receipt.json", proposal.proposal.id),
    ))
}

async fn export_csv(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Response, AppError> {
    valid_token(&token)?;
    let managed = db::by_manage(&state.pool, &token_hash(&state.secret, &token)).await?;
    let p = &managed.proposal;
    let mut csv = "proposal_id,title,item,description,quantity,unit_amount_cents,currency,decision,decided_at,receipt_hash\r\n".to_string();
    for item in &p.items {
        let decision = p.decision.as_ref();
        csv.push_str(
            &[
                csv_field(&p.id),
                csv_field(&p.title),
                csv_field(&item.label),
                csv_field(&item.description),
                item.quantity.to_string(),
                item.unit_amount_cents.to_string(),
                csv_field(&p.currency),
                csv_field(decision.map(|d| d.kind.as_str()).unwrap_or("pending")),
                csv_field(decision.map(|d| d.decided_at.as_str()).unwrap_or("")),
                csv_field(decision.map(|d| d.receipt_hash.as_str()).unwrap_or("")),
            ]
            .join(","),
        );
        csv.push_str("\r\n");
    }
    Ok(download(
        csv.into_bytes(),
        "text/csv; charset=utf-8",
        &format!("{}-receipt.csv", p.id),
    ))
}

#[derive(Deserialize)]
struct DeleteInput {
    confirmation: String,
}
async fn delete_proposal(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Json(input): Json<DeleteInput>,
) -> Result<StatusCode, AppError> {
    valid_token(&token)?;
    if input.confirmation != "DELETE" {
        return Err(AppError::BadRequest(
            "Type DELETE to confirm permanent deletion.".into(),
        ));
    }
    db::delete_by_manage(&state.pool, &token_hash(&state.secret, &token)).await?;
    Ok(StatusCode::NO_CONTENT)
}

fn download(body: Vec<u8>, content_type: &'static str, filename: &str) -> Response {
    let disposition = format!("attachment; filename=\"{filename}\"");
    (
        [
            (header::CONTENT_TYPE, content_type),
            (header::CONTENT_DISPOSITION, disposition.as_str()),
        ],
        body,
    )
        .into_response()
}
fn csv_field(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}
fn token() -> String {
    let mut bytes = [0_u8; 24];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}
fn valid_token(value: &str) -> Result<(), AppError> {
    if value.len() == 32
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        Ok(())
    } else {
        Err(AppError::NotFound)
    }
}
fn token_hash(secret: &[u8], value: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("HMAC supports any key size");
    mac.update(value.as_bytes());
    URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
}

async fn load_secret(data_dir: &FilePath) -> anyhow::Result<(Vec<u8>, &'static str)> {
    if let Ok(value) = std::env::var("INSTANCE_SECRET") {
        return Ok((value.into_bytes(), "supplied"));
    }
    let path = data_dir.join("instance-secret");
    if let Ok(value) = fs::read(&path).await {
        return Ok((value, "persisted"));
    }
    let mut value = vec![0_u8; 32];
    OsRng.fill_bytes(&mut value);
    fs::write(&path, &value).await?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).await?;
    }
    Ok((value, "generated"))
}

async fn security_headers(request: Request<Body>, next: Next) -> Response {
    let path = request.uri().path().to_owned();
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
    headers.insert(HeaderName::from_static("content-security-policy"), HeaderValue::from_static("default-src 'self'; connect-src 'self' https://api.sociobot.in; img-src 'self'; style-src 'self' 'unsafe-inline'; script-src 'self'; font-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'self' https://api.sociobot.in"));
    let cache = if path.starts_with("/assets/") {
        "public, max-age=31536000, immutable"
    } else if path.ends_with(".webp") || path.ends_with(".svg") {
        "public, max-age=86400"
    } else {
        "no-cache"
    };
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static(cache));
    response
}

async fn shutdown() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("install Ctrl+C handler")
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install signal handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
    tracing::info!("graceful shutdown started");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use tower::ServiceExt;

    async fn test_app() -> Router {
        let dir = tempfile::tempdir().unwrap();
        let db_url = format!("sqlite://{}", dir.path().join("test.sqlite").display());
        let pool = db::connect(&db_url).await.unwrap();
        std::mem::forget(dir);
        router(
            AppState {
                pool,
                secret: Arc::new(vec![7; 32]),
                mailer: None,
            },
            "missing-dist",
        )
    }

    #[tokio::test]
    async fn creates_reads_and_decides_once() {
        let app = test_app().await;
        let input = json!({"title":"Website refresh","freelancerName":"Fern Studio","freelancerEmail":"hello@fern.test","clientName":"Acme Client","clientEmail":"client@acme.test","message":"Scope below","currency":"USD","items":[{"label":"Discovery","description":"Workshop","quantity":1,"unitAmountCents":50000}]});
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/proposals")
                    .header("content-type", "application/json")
                    .header("x-forwarded-for", "192.0.2.1")
                    .body(Body::from(input.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 100_000).await.unwrap())
                .unwrap();
        let path = body["clientPath"].as_str().unwrap();
        let get = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/proposals/{}", path.trim_start_matches("/p/")))
                    .header("x-forwarded-for", "192.0.2.2")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(get.status(), StatusCode::OK);
        let decision = json!({"kind":"accepted","respondentName":"A Client","respondentEmail":"client@acme.test","note":"Proceed"});
        let decide_uri = format!("/api/proposals/{}/decision", path.trim_start_matches("/p/"));
        let first = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&decide_uri)
                    .header("content-type", "application/json")
                    .header("x-forwarded-for", "192.0.2.3")
                    .body(Body::from(decision.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(first.status(), StatusCode::CREATED);
        let second = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&decide_uri)
                    .header("content-type", "application/json")
                    .header("x-forwarded-for", "192.0.2.4")
                    .body(Body::from(decision.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(second.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn write_limit_returns_429_and_retry_after() {
        let app = test_app().await;
        let mut limited = None;
        for _ in 0..8 {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/proposals")
                        .header("content-type", "application/json")
                        .header("x-forwarded-for", "198.51.100.9")
                        .body(Body::from("{}"))
                        .unwrap(),
                )
                .await
                .unwrap();
            if response.status() == StatusCode::TOO_MANY_REQUESTS {
                limited = Some(response);
                break;
            }
        }
        let response = limited.expect("burst should be rate limited");
        assert!(response.headers().contains_key(header::RETRY_AFTER));
    }
}
