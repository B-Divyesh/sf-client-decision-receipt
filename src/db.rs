use crate::{
    error::AppError,
    model::{
        CreateProposal, Decision, Delivery, DemoWorkspace, LineItem, ManagedProposal, Proposal,
        SubmitDecision,
    },
};
use chrono::{Duration, SecondsFormat, Utc};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{sqlite::SqliteConnectOptions, Row, SqliteConnection, SqlitePool};
use std::time::Duration as StdDuration;

pub async fn connect(url: &str) -> anyhow::Result<SqlitePool> {
    let options = url
        .parse::<SqliteConnectOptions>()?
        .create_if_missing(true)
        .foreign_keys(true)
        // A second final decision waits for the first short write transaction,
        // then reads the recorded decision and returns 409. Without this,
        // SQLite can surface a transient lock as a misleading 500.
        .busy_timeout(StdDuration::from_secs(5));
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(10)
        .connect_with(options)
        .await?;
    for attempt in 1..=12 {
        match sqlx::raw_sql(include_str!("../migrations/0001_init.sql"))
            .execute(&pool)
            .await
        {
            Ok(_) => return Ok(pool),
            Err(error) if database_locked(&error) && attempt < 12 => {
                tracing::warn!(
                    attempt,
                    "database is busy during startup; retrying migration"
                );
                tokio::time::sleep(StdDuration::from_secs(1)).await;
            }
            Err(error) => return Err(error.into()),
        }
    }
    unreachable!("startup retry loop always returns")
}

fn database_locked(error: &sqlx::Error) -> bool {
    match error {
        sqlx::Error::Database(database) => {
            database
                .code()
                .is_some_and(|code| code == "5" || code == "6")
                || database.message().contains("database is locked")
                || database.message().contains("database is busy")
        }
        _ => false,
    }
}

pub async fn create_demo(pool: &SqlitePool, id: &str) -> Result<DemoWorkspace, AppError> {
    clean_expired_demos(pool).await?;
    let created_at = now();
    let expires_at = (Utc::now() + Duration::hours(24)).to_rfc3339_opts(SecondsFormat::Secs, true);
    sqlx::query("INSERT INTO demo_sessions (id, created_at, expires_at) VALUES (?, ?, ?)")
        .bind(id)
        .bind(&created_at)
        .bind(&expires_at)
        .execute(pool)
        .await?;
    demo_by_id(pool, id).await
}

pub async fn demo_by_id(pool: &SqlitePool, id: &str) -> Result<DemoWorkspace, AppError> {
    let row = sqlx::query("SELECT * FROM demo_sessions WHERE id = ? AND expires_at > ?")
        .bind(id)
        .bind(now())
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)?;
    demo_row_to_workspace(&row)
}

pub async fn decide_demo(
    pool: &SqlitePool,
    id: &str,
    input: &SubmitDecision,
) -> Result<DemoWorkspace, AppError> {
    let mut connection = begin_immediate(pool).await.map_err(write_conflict)?;
    let outcome = async {
        let row = sqlx::query("SELECT * FROM demo_sessions WHERE id = ? AND expires_at > ?")
            .bind(id)
            .bind(now())
            .fetch_optional(&mut *connection)
            .await?
            .ok_or(AppError::NotFound)?;
        if row.try_get::<Option<String>, _>("decision_kind")?.is_some() {
            return Err(AppError::Conflict);
        }
        let proposal = demo_row_to_workspace(&row)?.proposal;
        let decided_at = now();
        let receipt_hash = receipt_hash(&proposal, input, &decided_at)?;
        let changed = sqlx::query("UPDATE demo_sessions SET decision_kind=?, respondent_name=?, respondent_email=?, decision_note=?, decided_at=?, receipt_hash=? WHERE id=? AND decision_kind IS NULL AND expires_at > ?")
            .bind(&input.kind).bind(input.respondent_name.trim()).bind(input.respondent_email.trim().to_lowercase()).bind(input.note.trim())
            .bind(&decided_at).bind(&receipt_hash).bind(id).bind(now())
            .execute(&mut *connection).await?.rows_affected();
        if changed != 1 {
            return Err(AppError::Conflict);
        }
        Ok(())
    }
    .await;
    finish_transaction(&mut connection, outcome.is_ok()).await?;
    outcome?;
    demo_by_id(pool, id).await
}

pub async fn delete_demo(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let deleted = sqlx::query("DELETE FROM demo_sessions WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected();
    if deleted == 1 {
        Ok(())
    } else {
        Err(AppError::NotFound)
    }
}

async fn clean_expired_demos(pool: &SqlitePool) -> Result<(), AppError> {
    sqlx::query("DELETE FROM demo_sessions WHERE expires_at <= ?")
        .bind(now())
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn insert_proposal(
    pool: &SqlitePool,
    id: &str,
    client_token: &str,
    client_hash: &str,
    manage_hash: &str,
    input: &CreateProposal,
) -> Result<(), AppError> {
    sqlx::query("INSERT INTO proposals (id, client_token, client_token_hash, manage_token_hash, title, freelancer_name, freelancer_email, client_name, client_email, message, currency, items_json, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(id).bind(client_token).bind(client_hash).bind(manage_hash)
        .bind(input.title.trim()).bind(input.freelancer_name.trim()).bind(input.freelancer_email.trim().to_lowercase())
        .bind(input.client_name.trim()).bind(input.client_email.trim().to_lowercase()).bind(input.message.trim())
        .bind(&input.currency).bind(serde_json::to_string(&input.items).map_err(anyhow::Error::from)?)
        .bind(now()).execute(pool).await?;
    Ok(())
}

pub async fn by_client(pool: &SqlitePool, hash: &str) -> Result<Proposal, AppError> {
    let row = sqlx::query("SELECT * FROM proposals WHERE client_token_hash = ?")
        .bind(hash)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)?;
    row_to_proposal(&row, false)
}

pub async fn by_manage(pool: &SqlitePool, hash: &str) -> Result<ManagedProposal, AppError> {
    let row = sqlx::query("SELECT * FROM proposals WHERE manage_token_hash = ?")
        .bind(hash)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)?;
    let proposal = row_to_proposal(&row, true)?;
    let client_token: String = row.try_get("client_token")?;
    let deliveries = sqlx::query(
        "SELECT recipient, status, created_at FROM deliveries WHERE proposal_id = ? ORDER BY id",
    )
    .bind(&proposal.id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| Delivery {
        recipient: r.get("recipient"),
        status: r.get("status"),
        created_at: r.get("created_at"),
    })
    .collect();
    Ok(ManagedProposal {
        proposal,
        client_path: format!("/p/{client_token}"),
        delivery: deliveries,
    })
}

pub async fn decide(
    pool: &SqlitePool,
    client_hash: &str,
    input: &SubmitDecision,
    delivery_enabled: bool,
) -> Result<Proposal, AppError> {
    let mut connection = begin_immediate(pool).await.map_err(write_conflict)?;
    let outcome = async {
    let row = sqlx::query("SELECT * FROM proposals WHERE client_token_hash = ?")
        .bind(client_hash)
        .fetch_optional(&mut *connection)
        .await?
        .ok_or(AppError::NotFound)?;
    if row.try_get::<Option<String>, _>("decision_kind")?.is_some() {
        return Err(AppError::Conflict);
    }
    let proposal = row_to_proposal(&row, true)?;
    let decided_at = now();
    let receipt_hash = receipt_hash(&proposal, input, &decided_at)?;
    let changed = sqlx::query("UPDATE proposals SET decision_kind=?, respondent_name=?, respondent_email=?, decision_note=?, decided_at=?, receipt_hash=? WHERE id=? AND decision_kind IS NULL")
        .bind(&input.kind).bind(input.respondent_name.trim()).bind(input.respondent_email.trim().to_lowercase()).bind(input.note.trim())
        .bind(&decided_at).bind(&receipt_hash).bind(&proposal.id).execute(&mut *connection).await?.rows_affected();
    if changed != 1 {
        return Err(AppError::Conflict);
    }
    let decision_word = match input.kind.as_str() {
        "accepted" => "Accepted",
        "changes_requested" => "Changes requested",
        _ => "Declined",
    };
    let scope = proposal
        .items
        .iter()
        .map(|item| {
            format!(
                "- {} — {} × {} {}",
                item.label, item.quantity, item.unit_amount_cents, proposal.currency
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let body = format!("Decision receipt {}\n\nProposal: {}\nDecision: {}\nRecorded: {}\n\nFrozen scope (unit amounts in minor currency units):\n{}\n\nDecision note: {}\nReceipt SHA-256: {}\n\n{}", proposal.id, proposal.title, decision_word, decided_at, scope, input.note.trim(), receipt_hash, "This is a decision acknowledgement and scope record, not a regulated electronic signature.");
    let delivery_status = if delivery_enabled {
        "queued"
    } else {
        "sender_not_configured"
    };
    for recipient in [
        proposal.freelancer_email.as_deref().unwrap_or_default(),
        proposal.client_email.as_deref().unwrap_or_default(),
    ] {
        sqlx::query("INSERT INTO deliveries (proposal_id, recipient, status, subject, body, created_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(&proposal.id).bind(recipient).bind(delivery_status).bind(format!("Decision receipt: {} — {}", proposal.title, decision_word)).bind(&body).bind(&decided_at).execute(&mut *connection).await?;
    }
    Ok(())
    }.await;
    finish_transaction(&mut connection, outcome.is_ok()).await?;
    outcome?;
    by_client(pool, client_hash).await
}

async fn begin_immediate(
    pool: &SqlitePool,
) -> Result<sqlx::pool::PoolConnection<sqlx::Sqlite>, sqlx::Error> {
    let mut connection = pool.acquire().await?;
    sqlx::query("BEGIN IMMEDIATE")
        .execute(&mut *connection)
        .await?;
    Ok(connection)
}

fn write_conflict(error: sqlx::Error) -> AppError {
    if database_locked(&error) {
        return AppError::Conflict;
    }
    error.into()
}

async fn finish_transaction(
    connection: &mut SqliteConnection,
    commit: bool,
) -> Result<(), AppError> {
    sqlx::query(if commit { "COMMIT" } else { "ROLLBACK" })
        .execute(connection)
        .await?;
    Ok(())
}

pub async fn delete_by_manage(pool: &SqlitePool, manage_hash: &str) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    let row = sqlx::query(
        "SELECT id, receipt_hash, decided_at FROM proposals WHERE manage_token_hash = ?",
    )
    .bind(manage_hash)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;
    let id: String = row.get("id");
    if let (Some(receipt_hash), Some(decided_at)) = (
        row.try_get::<Option<String>, _>("receipt_hash")?,
        row.try_get::<Option<String>, _>("decided_at")?,
    ) {
        sqlx::query("INSERT OR IGNORE INTO receipt_tombstones (receipt_hash, proposal_id, decided_at, deleted_at) VALUES (?, ?, ?, ?)")
            .bind(receipt_hash).bind(&id).bind(decided_at).bind(now()).execute(&mut *tx).await?;
    }
    sqlx::query("DELETE FROM proposals WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

fn row_to_proposal(row: &sqlx::sqlite::SqliteRow, private: bool) -> Result<Proposal, AppError> {
    let items_json: String = row.try_get("items_json")?;
    let decision_kind: Option<String> = row.try_get("decision_kind")?;
    let decision = decision_kind.map(|kind| Decision {
        kind,
        respondent_name: row.get("respondent_name"),
        respondent_email: row.get("respondent_email"),
        note: row.get("decision_note"),
        decided_at: row.get("decided_at"),
        receipt_hash: row.get("receipt_hash"),
    });
    Ok(Proposal {
        id: row.get("id"),
        title: row.get("title"),
        freelancer_name: row.get("freelancer_name"),
        freelancer_email: private.then(|| row.get("freelancer_email")),
        client_name: row.get("client_name"),
        client_email: private.then(|| row.get("client_email")),
        message: row.get("message"),
        currency: row.get("currency"),
        items: serde_json::from_str::<Vec<LineItem>>(&items_json).map_err(anyhow::Error::from)?,
        created_at: row.get("created_at"),
        decision,
    })
}

fn demo_row_to_workspace(row: &sqlx::sqlite::SqliteRow) -> Result<DemoWorkspace, AppError> {
    let created_at: String = row.get("created_at");
    let decision_kind: Option<String> = row.try_get("decision_kind")?;
    let decision = decision_kind.map(|kind| Decision {
        kind,
        respondent_name: row.get("respondent_name"),
        respondent_email: row.get("respondent_email"),
        note: row.get("decision_note"),
        decided_at: row.get("decided_at"),
        receipt_hash: row.get("receipt_hash"),
    });
    Ok(DemoWorkspace {
        id: row.get("id"),
        expires_at: row.get("expires_at"),
        proposal: Proposal {
            id: "CDR-DEMO-FERN".into(),
            title: "Spring website refresh".into(),
            freelancer_name: "Fern Studio".into(),
            freelancer_email: Some("hello@fernstudio.example".into()),
            client_name: "Moss & Morning".into(),
            client_email: Some("team@mossandmorning.example".into()),
            message: "This sample shows the scope a client reviews before work starts.".into(),
            currency: "USD".into(),
            items: vec![
                LineItem {
                    label: "Planning workshop".into(),
                    description: "A 90-minute project workshop".into(),
                    quantity: 1,
                    unit_amount_cents: 65000,
                },
                LineItem {
                    label: "Homepage design".into(),
                    description: "Desktop and mobile design direction".into(),
                    quantity: 1,
                    unit_amount_cents: 180000,
                },
                LineItem {
                    label: "Build handoff".into(),
                    description: "Annotated files and implementation notes".into(),
                    quantity: 1,
                    unit_amount_cents: 75000,
                },
            ],
            created_at,
            decision,
        },
    })
}

fn receipt_hash(
    proposal: &Proposal,
    input: &SubmitDecision,
    decided_at: &str,
) -> Result<String, AppError> {
    let canonical = json!({
        "version": 1, "proposalId": proposal.id, "title": proposal.title,
        "freelancer": { "name": proposal.freelancer_name, "email": proposal.freelancer_email },
        "client": { "name": proposal.client_name, "email": proposal.client_email },
        "currency": proposal.currency, "items": proposal.items, "proposalCreatedAt": proposal.created_at,
        "decision": { "kind": input.kind, "respondentName": input.respondent_name.trim(), "respondentEmail": input.respondent_email.trim().to_lowercase(), "note": input.note.trim(), "decidedAt": decided_at }
    });
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&canonical).map_err(anyhow::Error::from)?)
    ))
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}
