use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;

#[derive(Clone, Debug, Error)]
#[error("{detail}")]
pub struct Error {
    pub code: &'static str,
    pub detail: &'static str,
    pub status: StatusCode,
}
pub type Result<T> = std::result::Result<T, Error>;
impl Error {
    pub fn new(code: &'static str, detail: &'static str, status: StatusCode) -> Self {
        Self {
            code,
            detail,
            status,
        }
    }
    pub fn invalid() -> Self {
        Self::new(
            "invalid_input",
            "Check the request fields and input length.",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
    }
    pub fn storage() -> Self {
        Self::new(
            "storage",
            "Local storage is temporarily unavailable. Retry shortly.",
            StatusCode::SERVICE_UNAVAILABLE,
        )
    }
    pub fn busy() -> Self {
        Self::new(
            "busy",
            "A response is already running on this device.",
            StatusCode::CONFLICT,
        )
    }
    pub fn protocol() -> Self {
        Self::new(
            "model_protocol",
            "The model returned an incomplete or invalid response. Check its tool-aware chat template.",
            StatusCode::BAD_GATEWAY,
        )
    }
    pub fn bad_turn(code: &'static str, detail: &'static str) -> Self {
        Self::new(code, detail, StatusCode::UNPROCESSABLE_ENTITY)
    }
}
impl From<rusqlite::Error> for Error {
    fn from(_: rusqlite::Error) -> Self {
        Self::storage()
    }
}
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(json!({"code":self.code,"detail":self.detail})),
        )
            .into_response()
    }
}
