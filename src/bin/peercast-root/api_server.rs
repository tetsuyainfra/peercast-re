use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{mpsc::channel, Arc},
};

use axum::{
    extract::{MatchedPath, Path, State},
    http::{HeaderMap, Request},
    response::{Html, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use http::StatusCode;
use peercast_re::pcp::{decode::PcpTrackInfo, GnuId, TrackInfo};
use peercast_re_api::{apis::urlencode, models::ChannelTrack};
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

use crate::{
    channel::{TrackerChannel, TrackerDetail},
    error::AppError,
    ChannelStore,
};

////////////////////////////////////////////////////////////////////////////////
/// API Server
///
pub async fn start_api_server(
    arc_channel_manager: Arc<ChannelStore<TrackerChannel>>,
    listener: tokio::net::TcpListener,
) {
    info!("START API SERVER");

    // let user_repo = Arc::new(ExampleUserRepo) as DynUserRepo;

    // let app = Router::new().route("/", get()).with_state(user_repo);
    let app = Router::new()
        .route("/", get(index))
        .route("/status", get(status))
        .route("/channels", get(channels))
        .route("/index.txt", get(index_txt))
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
            <a href='/channels'>/channels</a></br>
            <a href='/index.txt'>/index.txt</a></br>
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
    State(manager): State<Arc<ChannelStore<TrackerChannel>>>,
) -> Result<Json<Vec<JsonChannel>>, AppError> {
    let channels: Vec<JsonChannel> = manager
        .get_channels()
        .into_iter()
        .map(|c| c.detail().into())
        .collect();
    Ok(channels.into())
}

async fn index_txt(
    State(manager): State<Arc<ChannelStore<TrackerChannel>>>,
) -> (StatusCode, String) {
    let body = manager
        .get_channels()
        .into_iter()
        .map(|c| c.detail().into())
        .map(|c: JsonChannel| {
            create_index_line(
                c.name,
                c.id,
                c.addr,
                c.contact_url,
                c.genre,
                c.desc,
                c.number_of_listener,
                c.number_of_relay,
                c.bitrate,
                c.stream_ext,
                c.created_at,
                c.comment,
            )
        })
        .collect::<Vec<String>>()
        .join("\n");

    let notice = "";
    let code = StatusCode::ACCEPTED;
    (code, body)
}

fn create_index_line(
    name: String,
    id: GnuId,
    addr: SocketAddr,
    contact_url: String,
    genre: String,
    desc: String,
    number_of_listener: i32,
    number_of_relay: i32,
    bitrate: i32,
    stream_ext: String,
    created_at: DateTime<Utc>,
    comment: String,
) -> String {
    let diff_time = Utc::now() - created_at;
    let hour = diff_time.num_hours();
    let min = diff_time.num_minutes() % 60;

    format!(
        "{name}<>{id}<>{addr}<>{contact_url}<>{genre}<>\
        {desc}<>{number_of_listener}<>{number_of_relay}<>{bitrate}<>{file_ext}<>\
        <><><><>{name_escaped}<>{time_hour}:{time_min:02}<>click<>{comment}<>",
        name = name.clone(),
        id = id,
        addr = addr,
        contact_url = contact_url,
        genre = genre,
        desc = desc,
        number_of_listener = number_of_listener,
        number_of_relay = number_of_relay,
        bitrate = bitrate,
        file_ext = stream_ext,
        name_escaped = urlencode(name),
        time_hour = hour,
        time_min = min,
        comment = comment
    )
}

#[derive(Debug, Serialize)]
struct AppStatus {
    x_request_id: String,
}

#[derive(Debug, Serialize)]
struct JsonChannel {
    id: GnuId,
    name: String,
    addr: SocketAddr,
    contact_url: String,
    genre: String,
    desc: String,
    comment: String,
    stream_type: String,
    stream_ext: String,
    bitrate: i32,
    // filetype: String,
    // status: String,
    number_of_listener: i32,
    number_of_relay: i32,
    created_at: DateTime<Utc>,
    track: JsonTrack,
}

impl From<TrackerDetail> for JsonChannel {
    fn from(detail: TrackerDetail) -> Self {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

        let channel_info = detail.channel_info;
        let track_info = detail.track_info;
        let created_at = detail.created_at.as_ref().clone();
        JsonChannel {
            id: detail.id.as_ref().clone(),
            //
            name: channel_info.name(),
            contact_url: channel_info.url(),
            genre: channel_info.genre(),
            desc: channel_info.desc(),
            comment: channel_info.comment(),
            stream_type: channel_info.stream_type(),
            stream_ext: channel_info.stream_ext(),
            bitrate: channel_info.bitrate(),
            //
            track: track_info.into(),
            //
            addr,
            number_of_listener: 100_i32,
            number_of_relay: 200_i32,
            created_at,
        }
    }
}

#[derive(Debug, Serialize)]
struct JsonTrack {
    title: String,
    creator: String,
    url: String,
    album: String,
    genre: String,
}

impl From<PcpTrackInfo> for JsonTrack {
    fn from(track_info: PcpTrackInfo) -> Self {
        JsonTrack {
            title: track_info.title(),
            creator: track_info.creator(),
            url: track_info.url(),
            album: track_info.album(),
            genre: track_info.genre(),
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
