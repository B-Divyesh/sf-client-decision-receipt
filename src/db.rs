use crate::{
    error::AppError,
    model::{
        CreateProposal, Decision, Delivery, LineItem, ManagedProposal, Proposal, SubmitDecision,
    },
};
use chrono::{SecondsFormat, Utc};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool};

pub async fn connect(url: &str) -> anyhow::Result<SqlitePool> {
    let options = url
        .parse::<sqlx::sqlite::SqliteConnectOptions>()?
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(10)
        .connect_with(options)
        .await?;
    sqlx::raw_sql(include_str!("../migrations/0001_init.sql"))
        .execute(&pool)
        .await?;
    Ok(pool)
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
) -> Result<Proposal, AppError> {
    let mut tx = pool.begin().await?;
    let row = sqlx::query("SELECT * FROM proposals WHERE client_token_hash = ?")
        .bind(client_hash)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(AppError::NotFound)?;
    if row.try_get::<Option<String>, _>("decision_kind")?.is_some() {
        return Err(AppError::Conflict);
    }
    let proposal = row_to_proposal(&row, true)?;
    let decided_at = now();
    let canonical = json!({
        "version": 1, "proposalId": proposal.id, "title": proposal.title,
        "freelancer": { "name": proposal.freelancer_name, "email": proposal.freelancer_email },
        "client": { "name": proposal.client_name, "email": proposal.client_email },
        "currency": proposal.currency, "items": proposal.items, "proposalCreatedAt": proposal.created_at,
        "decision": { "kind": input.kind, "respondentName": input.respondent_name.trim(), "respondentEmail": input.respondent_email.trim().to_lowercase(), "note": input.note.trim(), "decidedAt": decided_at }
    });
    let receipt_hash = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&canonical).map_err(anyhow::Error::from)?)
    );
    let changed = sqlx::query("UPDATE proposals SET decision_kind=?, respondent_name=?, respondent_email=?, decision_note=?, decided_at=?, receipt_hash=? WHERE id=? AND decision_kind IS NULL")
        .bind(&input.kind).bind(input.respondent_name.trim()).bind(input.respondent_email.trim().to_lowercase()).bind(input.note.trim())
        .bind(&decided_at).bind(&receipt_hash).bind(&proposal.id).execute(&mut *tx).await?.rows_affected();
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
    for recipient in [
        proposal.freelancer_email.as_deref().unwrap_or_default(),
        proposal.client_email.as_deref().unwrap_or_default(),
    ] {
        sqlx::query("INSERT INTO deliveries (proposal_id, recipient, status, subject, body, created_at) VALUES (?, ?, 'queued', ?, ?, ?)")
            .bind(&proposal.id).bind(recipient).bind(format!("Decision receipt: {} — {}", proposal.title, decision_word)).bind(&body).bind(&decided_at).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    by_client(pool, client_hash).await
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

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}
