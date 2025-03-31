use axum::{extract::State, routing::get, Json, Router};
use axum_core::response::IntoResponse;

use crate::http::AppState;

pub(super) struct ConfigSvc;

impl ConfigSvc {
    pub(super) fn new() -> Router<AppState> {
        Router::new().route("/", get(get_config).put(put_config))
    }
}

async fn get_config(State(state): State<AppState>) -> impl IntoResponse {
    state.config.to_string()
}
async fn put_config(State(state): State<AppState>) -> impl IntoResponse {
    ()
}
