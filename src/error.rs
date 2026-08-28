use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),
    #[error("The proposal link was not found or has been deleted.")]
    NotFound,
    #[error("A decision has already been recorded for this proposal.")]
    Conflict,
    #[error("Internal service error")]
    Internal(#[from] anyhow::Error),
}

impl From<sqlx::Error> for AppError {
    fn from(value: sqlx::Error) -> Self { Self::Internal(value.into()) }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, message.as_str()),
            Self::NotFound => (StatusCode::NOT_FOUND, "The proposal link was not found or has been deleted."),
            Self::Conflict => (StatusCode::CONFLICT, "A decision has already been recorded for this proposal."),
            Self::Internal(error) => {
                tracing::error!(error = ?error, "request failed");
                (StatusCode::INTERNAL_SERVER_ERROR, "The service could not complete that request. Please try again.")
            }
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}
