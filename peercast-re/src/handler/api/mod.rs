// src/handler/api/mod.rs
// API handler
//
use crate::{AppState, SWAGGER_PATH};

pub mod v1;
pub mod v2;

pub fn build_router() -> axum::Router<AppState> {
    axum::Router::new()
        //
        .nest("/v1", v1::router())
        .nest("/v2", v2::router())
}
