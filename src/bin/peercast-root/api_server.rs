use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{mpsc::channel, Arc},
};

use async_trait::async_trait;
use axum::{
    extract::{MatchedPath, Path, State},
    http::{HeaderMap, Request},
    response::{Html, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use peercast_re::pcp::GnuId;
use peercast_re_api::models::ChannelTrack;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::{
    request_id::MakeRequestUuid,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
    ServiceBuilderExt,
};
use tracing::{info, info_span, Span};
use uuid::Uuid;

use crate::{channel::TrackerChannel, error::AppError, ChannelManager};

////////////////////////////////////////////////////////////////////////////////
/// API Server
///
pub async fn start_api_server(
    arc_channel_manager: Arc<ChannelManager<TrackerChannel>>,
    listener: tokio::net::TcpListener,
) {
    info!("START API SERVER");

    // let user_repo = Arc::new(ExampleUserRepo) as DynUserRepo;

    // let app = Router::new().route("/", get()).with_state(user_repo);
    let app = Router::new()
        .route("/", get(index))
        .route("/status", get(status))
        .route("/channels", get(channels))
        // ServiceBuilderを使ってlayerを定義した方が良いらしい：理由は不明
        // https://github.com/tokio-rs/axum/blob/main/axum/src/docs/middleware.md#applying-multiple-middleware
        .layer(
            ServiceBuilder::new()
                .set_x_request_id(MakeRequestUuid::default())
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(DefaultMakeSpan::new().include_headers(true))
                        .on_response(DefaultOnResponse::new().include_headers(true)),
                )
                .propagate_x_request_id(),
        )
        .with_state(arc_channel_manager);

    axum::serve(listener, app).await.unwrap();
}

async fn index() -> Result<Html<&'static str>, AppError> {
    Ok(Html(
        "<div>
        <h1>index</h1>
        <p>
            <a href='/status'>/status</a></br>
            <a href='/channels'>/channels</a>
        </p>
        </div>
    ",
    ))
}
async fn status(headers: HeaderMap) -> Result<Json<AppStatus>, AppError> {
    info!("status!!!");
    let x_id = headers.get("x-request-id").unwrap().to_str().unwrap();
    info!("x_id: {x_id:?}");
    let status: AppStatus = AppStatus {
        x_request_id: x_id.into(),
    };
    Ok(status.into())
}

async fn channels(
    State(manager): State<Arc<ChannelManager<TrackerChannel>>>,
) -> Result<Json<Vec<JsonChannel>>, AppError> {
    let channels: Vec<JsonChannel> = manager
        .get_channels()
        .into_iter()
        .map(|c| c.into())
        .collect();
    Ok(channels.into())
}

#[derive(Debug, Serialize)]
struct AppStatus {
    x_request_id: String,
}

#[derive(Debug, Serialize)]
struct JsonChannel {
    id: GnuId,
    title: String,
    title_escaped: String,
    addr: SocketAddr,
    contact_url: String,
    genre: String,
    desc: String,
    number_of_listener: i32,
    number_of_relay: i32,
    bitrate: i32,
    filetype: String,
    status: String,
    comment: String,
    created_at: DateTime<Utc>,
}

impl From<TrackerChannel> for JsonChannel {
    fn from(ch: TrackerChannel) -> Self {
        let _details = ch.detail_reciever.borrow().clone();

        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

        JsonChannel {
            id: ch.channel_id.as_ref().clone(),
            title: String::from("example"),
            title_escaped: String::from("escaped"),
            addr,
            contact_url: String::from("contact_url"),
            genre: String::from("genre"),
            desc: String::from("desc"),
            number_of_listener: 100_i32,
            number_of_relay: 200_i32,
            bitrate: 1024_i32,
            filetype: String::from("filetype"),
            status: String::from("status"),
            comment: String::from("comment"),
            created_at: ch.created_at.as_ref().clone(),
        }
    }
}

/*
async fn users_show(
    Path(user_id): Path<Uuid>,
    State(user_repo): State<DynUserRepo>,
) -> Result<Json<User>, AppError> {
    let user = user_repo.find(user_id).await?;

    Ok(user.into())
}

/// Handler for `POST /users`.
async fn users_create(
    State(user_repo): State<DynUserRepo>,
    Json(params): Json<CreateUser>,
) -> Result<Json<User>, AppError> {
    let user = user_repo.create(params).await?;

    Ok(user.into())
}

/// Example implementation of `UserRepo`.
struct ExampleUserRepo;

#[async_trait]
impl UserRepo for ExampleUserRepo {
    async fn find(&self, _user_id: Uuid) -> Result<User, UserRepoError> {
        unimplemented!()
    }

    async fn create(&self, _params: CreateUser) -> Result<User, UserRepoError> {
        unimplemented!()
    }
}

/// Type alias that makes it easier to extract `UserRepo` trait objects.
type DynUserRepo = Arc<dyn UserRepo + Send + Sync>;

/// A trait that defines things a user repo might support.
#[async_trait]
trait UserRepo {
    /// Loop up a user by their id.
    async fn find(&self, user_id: Uuid) -> Result<User, UserRepoError>;

    /// Create a new user.
    async fn create(&self, params: CreateUser) -> Result<User, UserRepoError>;
}

#[derive(Debug, Serialize)]
struct User {
    id: Uuid,
    username: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct CreateUser {
    username: String,
}

 */
