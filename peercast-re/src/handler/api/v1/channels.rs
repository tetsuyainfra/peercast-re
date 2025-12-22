use axum::{extract::State, routing};

use crate::{AppState, prelude::*};

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        //
        .route("/", routing::get(list_channels))
}

#[instrument(skip(_store))]
async fn list_channels(State(_store): State<AppState>) -> impl axum::response::IntoResponse {
    "list channels"
}
