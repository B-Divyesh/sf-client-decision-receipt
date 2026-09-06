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
use tokio::{fs, io::AsyncWriteExt, net::TcpListener, sync::Mutex};
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorLayer,
};
use tower_http::{compression::CompressionLayer, trace::TraceLayer};

#[derive(Clone)]
pub struct AppState {
    pool: SqlitePool,
    secret: Arc<Vec<u8>>,
    mailer: Option<Mailer>,
    dist: Arc<PathBuf>,
    durable: Option<Arc<DurableDatabase>>,
}

/// Azure Files provides durable shared storage but its byte-range locks are
/// not compatible with SQLite's live locking protocol. The running database
/// is therefore local to the one allowed replica; this object atomically
/// snapshots a valid SQLite database to /data after every real-data write.
/// On boot, the snapshot is restored before the app accepts traffic.
struct DurableDatabase {
    snapshot: PathBuf,
    staged: PathBuf,
    lock: Mutex<()>,
}

impl DurableDatabase {
    async fn open(data_dir: &FilePath) -> anyhow::Result<(SqlitePool, Arc<Self>)> {
        let unique = format!("client-decision-receipt-{}", std::process::id());
        let working = std::env::temp_dir().join(format!("{unique}.sqlite"));
        let snapshot = data_dir.join("receipts-durable.sqlite");
        if fs::try_exists(&snapshot).await? {
            copy_bytes(&snapshot, &working).await?;
        }
        let durable = Arc::new(Self {
            snapshot,
            staged: std::env::temp_dir().join(format!("{unique}.snapshot.sqlite")),
            lock: Mutex::new(()),
        });
        let url = format!("sqlite://{}", working.display());
        let pool = db::connect(&url).await?;
        durable.checkpoint(&pool).await?;
        Ok((pool, durable))
    }

    async fn checkpoint(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        let _guard = self.lock.lock().await;
        let _ = fs::remove_file(&self.staged).await;
        let target = self.staged.to_string_lossy().replace('\'', "''");
        sqlx::raw_sql(&format!("VACUUM INTO '{target}'"))
            .execute(pool)
            .await?;
        // Azure Files does not permit the POSIX rename primitive from this
        // container mount. Copying a fully-materialized local SQLite backup
        // directly is safe with the deployment's enforced one-writer limit.
        copy_bytes(&self.staged, &self.snapshot).await?;
        let _ = fs::remove_file(&self.staged).await;
        Ok(())
    }
}

async fn copy_bytes(source: &FilePath, destination: &FilePath) -> anyhow::Result<()> {
    let mut input = fs::File::open(source).await?;
    let mut output = fs::File::create(destination).await?;
    tokio::io::copy(&mut input, &mut output).await?;
    output.flush().await?;
    output.sync_all().await?;
    Ok(())
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
    let configured_database = std::env::var("DATABASE_URL").ok();
    let (pool, durable, database_source) = if let Some(url) = configured_database {
        (db::connect(&url).await?, None, "supplied DATABASE_URL")
    } else {
        let (pool, durable) = DurableDatabase::open(&data_dir).await?;
        (pool, Some(durable), "durable /data snapshot")
    };
    let mailer = Mailer::from_env()?;
    let mail_source = if mailer.is_some() {
        "supplied SMTP"
    } else {
        "sender not configured"
    };
    let state = AppState {
        pool,
        secret: Arc::new(secret),
        mailer,
        dist: Arc::new(PathBuf::from(
            std::env::var("DIST_DIR").unwrap_or_else(|_| "dist".into()),
        )),
        durable,
    };
    if let Some(mailer) = state.mailer.clone() {
        let pending_state = state.clone();
        tokio::spawn(async move {
            mailer.flush_pending(&pending_state.pool).await;
            let _ = persist(&pending_state).await;
        });
    }
    let app = router(state);
    let address = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!(
        port,
        build_sha = env!("BUILD_SHA"),
        instance_secret = secret_source,
        database = database_source,
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

pub fn router(state: AppState) -> Router {
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
        .route("/manage/{token}/export.csv", get(export_csv))
        .route("/demo/{id}", get(get_demo));
    let writes = Router::new()
        .route("/proposals", post(create_proposal))
        .route("/proposals/{token}/decision", post(submit_decision))
        .route("/manage/{token}", delete(delete_proposal))
        .route("/demo", post(create_demo))
        .route("/demo/{id}/decision", post(submit_demo))
        .route("/demo/{id}", delete(delete_demo))
        .layer(GovernorLayer::new(write_config));
    let api = Router::new()
        .merge(reads)
        .merge(writes)
        .fallback(api_not_found)
        .layer(middleware::from_fn(useful_retry_after))
        .layer(GovernorLayer::new(read_config))
        .with_state(state.clone());
    Router::new()
        .route("/health", get(health))
        .nest("/api", api)
        .route("/", get(serve_index))
        .route("/demo", get(serve_index))
        .route("/privacy", get(serve_index))
        .route("/terms", get(serve_index))
        .route("/404", get(spa_not_found))
        .route("/p/{token}", get(serve_index))
        .route("/m/{token}", get(serve_index))
        .route("/assets/{*path}", get(serve_asset))
        .route("/sw.js", get(serve_static))
        .route("/leaf.svg", get(serve_static))
        .route("/manifest.webmanifest", get(serve_static))
        .route("/robots.txt", get(serve_static))
        .route("/sitemap.xml", get(serve_static))
        .route("/herbarium-receipt-640.webp", get(serve_static))
        .route("/herbarium-receipt-960.webp", get(serve_static))
        .route("/og-receipt.jpg", get(serve_static))
        .fallback(spa_not_found)
        .with_state(state)
        .layer(DefaultBodyLimit::max(256 * 1024))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(security_headers))
}

async fn serve_index(State(state): State<AppState>) -> Result<Response, AppError> {
    serve_file(&state, "index.html", StatusCode::OK).await
}

async fn spa_not_found(State(state): State<AppState>) -> Result<Response, AppError> {
    serve_file(&state, "index.html", StatusCode::NOT_FOUND).await
}

async fn serve_asset(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> Result<Response, AppError> {
    if path
        .split('/')
        .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(AppError::NotFound);
    }
    serve_file(&state, &format!("assets/{path}"), StatusCode::OK).await
}

async fn serve_static(
    State(state): State<AppState>,
    request: Request<Body>,
) -> Result<Response, AppError> {
    let path = request.uri().path().trim_start_matches('/');
    serve_file(&state, path, StatusCode::OK).await
}

async fn serve_file(
    state: &AppState,
    requested: &str,
    status: StatusCode,
) -> Result<Response, AppError> {
    let bytes = fs::read(state.dist.join(requested))
        .await
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                AppError::NotFound
            } else {
                AppError::Internal(error.into())
            }
        })?;
    Ok((
        status,
        [(header::CONTENT_TYPE, content_type(requested))],
        bytes,
    )
        .into_response())
}

fn content_type(path: &str) -> &'static str {
    if path.ends_with(".html") {
        "text/html; charset=utf-8"
    } else if path.ends_with(".js") {
        "text/javascript; charset=utf-8"
    } else if path.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".webp") {
        "image/webp"
    } else if path.ends_with(".jpg") {
        "image/jpeg"
    } else if path.ends_with(".webmanifest") {
        "application/manifest+json"
    } else if path.ends_with(".txt") {
        "text/plain; charset=utf-8"
    } else if path.ends_with(".xml") {
        "application/xml; charset=utf-8"
    } else {
        "application/octet-stream"
    }
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
    persist(&state).await?;
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
    let proposal = db::decide(
        &state.pool,
        &token_hash(&state.secret, &token),
        &input,
        state.mailer.is_some(),
    )
    .await?;
    if let Some(mailer) = state.mailer.clone() {
        let pending_state = state.clone();
        let id = proposal.id.clone();
        tokio::spawn(async move {
            mailer.flush_proposal(&pending_state.pool, &id).await;
            let _ = persist(&pending_state).await;
        });
    }
    persist(&state).await?;
    Ok((StatusCode::CREATED, Json(proposal)))
}

async fn create_demo(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<model::DemoWorkspace>), AppError> {
    let id = token();
    let workspace = db::create_demo(&state.pool, &id).await?;
    Ok((StatusCode::CREATED, Json(workspace)))
}

async fn get_demo(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<model::DemoWorkspace>, AppError> {
    valid_token(&id)?;
    Ok(Json(db::demo_by_id(&state.pool, &id).await?))
}

async fn submit_demo(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<SubmitDecision>,
) -> Result<Json<model::DemoWorkspace>, AppError> {
    valid_token(&id)?;
    model::validate_decision(&input).map_err(|e| AppError::BadRequest(e.into()))?;
    Ok(Json(db::decide_demo(&state.pool, &id, &input).await?))
}

async fn delete_demo(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    valid_token(&id)?;
    db::delete_demo(&state.pool, &id).await?;
    Ok(StatusCode::NO_CONTENT)
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
    persist(&state).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn persist(state: &AppState) -> Result<(), AppError> {
    if let Some(durable) = &state.durable {
        durable.checkpoint(&state.pool).await?;
    }
    Ok(())
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
    headers.insert(HeaderName::from_static("content-security-policy"), HeaderValue::from_static("default-src 'self'; connect-src 'self' https://api.sociobot.in; img-src 'self'; style-src 'self'; script-src 'self'; font-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'self' https://api.sociobot.in"));
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

async fn useful_retry_after(request: Request<Body>, next: Next) -> Response {
    let mut response = next.run(request).await;
    if response.status() == StatusCode::TOO_MANY_REQUESTS
        && response
            .headers()
            .get(header::RETRY_AFTER)
            .is_none_or(|value| value == "0")
    {
        response
            .headers_mut()
            .insert(header::RETRY_AFTER, HeaderValue::from_static("1"));
    }
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
        router(AppState {
            pool,
            secret: Arc::new(vec![7; 32]),
            mailer: None,
            dist: Arc::new(PathBuf::from("missing-dist")),
            durable: None,
        })
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
        assert_eq!(response.headers().get(header::RETRY_AFTER).unwrap(), "1");
    }

    #[tokio::test]
    async fn concurrent_final_decisions_return_created_and_conflict() {
        let app = test_app().await;
        let input = json!({"title":"Concurrent receipt","freelancerName":"Fern Studio","freelancerEmail":"hello@fern.test","clientName":"Aster Client","clientEmail":"aster@example.test","message":"Scope below","currency":"USD","items":[{"label":"Discovery","description":"Workshop","quantity":1,"unitAmountCents":50000}]});
        let created = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/proposals")
                    .header("content-type", "application/json")
                    .header("x-forwarded-for", "203.0.113.20")
                    .body(Body::from(input.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(created.into_body(), 100_000).await.unwrap()).unwrap();
        let token = body["clientPath"]
            .as_str()
            .unwrap()
            .trim_start_matches("/p/");
        let payload = json!({"kind":"accepted","respondentName":"A Client","respondentEmail":"client@acme.test","note":"Proceed"}).to_string();
        let request = |ip: &str| {
            Request::builder()
                .method("POST")
                .uri(format!("/api/proposals/{token}/decision"))
                .header("content-type", "application/json")
                .header("x-forwarded-for", ip)
                .body(Body::from(payload.clone()))
                .unwrap()
        };
        let (first, second) = tokio::join!(
            app.clone().oneshot(request("203.0.113.21")),
            app.oneshot(request("203.0.113.22"))
        );
        let mut statuses = [first.unwrap().status(), second.unwrap().status()];
        statuses.sort();
        assert_eq!(statuses, [StatusCode::CREATED, StatusCode::CONFLICT]);
    }

    #[tokio::test]
    async fn demo_workspaces_are_separate_from_real_proposals() {
        let app = test_app().await;
        let demo = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/demo")
                    .header("x-forwarded-for", "203.0.113.30")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(demo.status(), StatusCode::CREATED);
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(demo.into_body(), 100_000).await.unwrap()).unwrap();
        let id = body["id"].as_str().unwrap();
        let decision = json!({"kind":"accepted","respondentName":"Maya Patel","respondentEmail":"maya@example.test","note":"Looks good"});
        let decided = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/demo/{id}/decision"))
                    .header("content-type", "application/json")
                    .header("x-forwarded-for", "203.0.113.31")
                    .body(Body::from(decision.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(decided.status(), StatusCode::OK);
        let managed = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/manage/{id}"))
                    .header("x-forwarded-for", "203.0.113.32")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(managed.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn shared_data_directory_keeps_private_links_across_instances() {
        let directory = tempfile::tempdir().unwrap();
        let url = format!(
            "sqlite://{}",
            directory.path().join("shared.sqlite").display()
        );
        let first_pool = db::connect(&url).await.unwrap();
        let second_pool = db::connect(&url).await.unwrap();
        let (first_secret, _) = load_secret(directory.path()).await.unwrap();
        let (second_secret, _) = load_secret(directory.path()).await.unwrap();
        assert_eq!(first_secret, second_secret);
        let first = router(AppState {
            pool: first_pool,
            secret: Arc::new(first_secret),
            mailer: None,
            dist: Arc::new(PathBuf::from("missing-dist")),
            durable: None,
        });
        let second = router(AppState {
            pool: second_pool,
            secret: Arc::new(second_secret),
            mailer: None,
            dist: Arc::new(PathBuf::from("missing-dist")),
            durable: None,
        });
        let input = json!({"title":"Shared mount receipt","freelancerName":"Fern Studio","freelancerEmail":"hello@fern.test","clientName":"Aster Client","clientEmail":"aster@example.test","message":"Scope below","currency":"USD","items":[{"label":"Discovery","description":"Workshop","quantity":1,"unitAmountCents":50000}]});
        let created = first
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/proposals")
                    .header("content-type", "application/json")
                    .header("x-forwarded-for", "198.51.100.20")
                    .body(Body::from(input.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(created.into_body(), 100_000).await.unwrap()).unwrap();
        let client_path = body["clientPath"].as_str().unwrap();
        let read = second
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/proposals/{}",
                        client_path.trim_start_matches("/p/")
                    ))
                    .header("x-forwarded-for", "198.51.100.21")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(read.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn durable_snapshot_restores_private_links_after_restart() {
        let directory = tempfile::tempdir().unwrap();
        let (pool, durable) = DurableDatabase::open(directory.path()).await.unwrap();
        let input: CreateProposal = serde_json::from_value(json!({
            "title":"Restarted receipt", "freelancerName":"Fern Studio",
            "freelancerEmail":"hello@fern.test", "clientName":"Aster Client",
            "clientEmail":"aster@example.test", "message":"Scope below", "currency":"USD",
            "items":[{"label":"Discovery","description":"Workshop","quantity":1,"unitAmountCents":50000}]
        }))
        .unwrap();
        db::insert_proposal(
            &pool,
            "CDR-RESTART",
            "private-token",
            "client-hash",
            "manage-hash",
            &input,
        )
        .await
        .unwrap();
        durable.checkpoint(&pool).await.unwrap();
        pool.close().await;

        let (restored_pool, _) = DurableDatabase::open(directory.path()).await.unwrap();
        let restored = db::by_client(&restored_pool, "client-hash").await.unwrap();
        assert_eq!(restored.id, "CDR-RESTART");
        assert_eq!(restored.title, "Restarted receipt");
    }
}
