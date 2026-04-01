use std::time::SystemTime;

use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use axum_core::response::IntoResponse;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::AppState;
use crate::pcp::GnuId;

// mod channels;
mod config;

////////////////////////////////////////////////////////////////////////////////
// Api
//
pub struct Api;

impl Api {
    pub(super) fn new() -> Router<AppState> {
        Router::new()
            // .nest("/channels", channels::ChannelsSvc::new())
            .nest("/config", config::ConfigSvc::new())
            .route("/ping", get(Self::pong))
            .route("/info", get(Self::info))
    }

    async fn pong(query: Query<PingQuery>) -> impl IntoResponse {
        Json(RespPong {
            pong: query.name + 1,
        })
    }

    async fn info(State(app): State<AppState>) -> impl IntoResponse {
        Json(json!({
            "hostname": "localhost",
            "port": app.config.server_port,
        }))
    }
}

#[derive(Deserialize)]
struct PingQuery {
    name: u32,
}

#[derive(Serialize)]
struct RespPong {
    pong: u32,
}
