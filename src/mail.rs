use lettre::{
    transport::smtp::authentication::Credentials, AsyncSmtpTransport, AsyncTransport, Message,
    Tokio1Executor,
};
use sqlx::{Row, SqlitePool};

#[derive(Clone)]
pub struct Mailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: String,
}

impl Mailer {
    pub fn from_env() -> anyhow::Result<Option<Self>> {
        let Ok(host) = std::env::var("SMTP_HOST") else {
            return Ok(None);
        };
        let port = std::env::var("SMTP_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(587);
        let from = std::env::var("SMTP_FROM")
            .unwrap_or_else(|_| "receipts@client-decision-receipt.sociobot.in".into());
        let mut builder = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host)?.port(port);
        if let (Ok(username), Ok(password)) = (
            std::env::var("SMTP_USERNAME"),
            std::env::var("SMTP_PASSWORD"),
        ) {
            builder = builder.credentials(Credentials::new(username, password));
        }
        Ok(Some(Self {
            transport: builder.build(),
            from,
        }))
    }

    pub async fn flush_proposal(&self, pool: &SqlitePool, proposal_id: &str) {
        let rows = match sqlx::query("SELECT id, recipient, subject, body FROM deliveries WHERE proposal_id = ? AND status IN ('queued', 'retry_needed', 'sender_not_configured')").bind(proposal_id).fetch_all(pool).await {
            Ok(rows) => rows,
            Err(error) => { tracing::error!(?error, "failed to read delivery queue"); return; }
        };
        for row in rows {
            let id: i64 = row.get("id");
            let recipient: String = row.get("recipient");
            let message = Message::builder()
                .from(match self.from.parse() {
                    Ok(v) => v,
                    Err(error) => {
                        tracing::error!(?error, "invalid SMTP_FROM");
                        return;
                    }
                })
                .to(match recipient.parse() {
                    Ok(v) => v,
                    Err(error) => {
                        tracing::error!(?error, %recipient, "invalid recipient");
                        continue;
                    }
                })
                .subject(row.get::<String, _>("subject"))
                .body(row.get::<String, _>("body"));
            let status = match message {
                Ok(message) => {
                    if self.transport.send(message).await.is_ok() {
                        "sent"
                    } else {
                        "retry_needed"
                    }
                }
                Err(_) => "invalid_message",
            };
            if let Err(error) = sqlx::query("UPDATE deliveries SET status = ? WHERE id = ?")
                .bind(status)
                .bind(id)
                .execute(pool)
                .await
            {
                tracing::error!(?error, delivery_id = id, "failed to update delivery status");
            }
        }
    }

    pub async fn flush_pending(&self, pool: &SqlitePool) {
        let rows = match sqlx::query("SELECT DISTINCT proposal_id FROM deliveries WHERE status IN ('queued', 'retry_needed', 'sender_not_configured')").fetch_all(pool).await {
            Ok(rows) => rows,
            Err(error) => { tracing::error!(?error, "failed to scan delivery queue"); return; }
        };
        for row in rows {
            self.flush_proposal(pool, &row.get::<String, _>("proposal_id"))
                .await;
        }
    }
}
