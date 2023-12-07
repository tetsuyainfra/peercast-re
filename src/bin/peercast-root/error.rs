use axum::Json;
use axum_core::response::{IntoResponse, Response};
use hyper::StatusCode;
use serde_json::json;
// error.rs
use uuid::Uuid;

pub struct ApiError {
    pub error: AppError,
    pub req_id: Uuid,
}

// Errorの種類を列挙する
pub enum AppError {
    Generic { description: String },
    LoginFail,
    UserRepo(UserRepoError),
}

#[derive(Debug)]
pub enum UserRepoError {
    #[allow(dead_code)]
    NotFound,
    #[allow(dead_code)]
    InvalidUsername,
}

pub type ApiResult<T> = core::result::Result<T, ApiError>;

/// This makes it possible to use `?` to automatically convert a `UserRepoError`
/// into an `AppError`.
impl From<UserRepoError> for AppError {
    fn from(inner: UserRepoError) -> Self {
        AppError::UserRepo(inner)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::UserRepo(UserRepoError::NotFound) => {
                (StatusCode::NOT_FOUND, "User not found")
            }
            AppError::UserRepo(UserRepoError::InvalidUsername) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "Invalid username")
            }
            AppError::Generic { description } => todo!(),
            AppError::LoginFail => todo!(),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}
