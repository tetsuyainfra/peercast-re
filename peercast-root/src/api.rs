use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use axum::{
    Json, Router,
    extract::{FromRef, FromRequestParts, Query, State},
    http::HeaderValue,
    response::IntoResponse,
    routing,
};

use axum_client_ip::ClientIp;
use bb8::PooledConnection;
use chrono::{DateTime, TimeZone, Utc};
use futures_util::{FutureExt, future::BoxFuture, select};
use hyper::{Method, StatusCode};
use libpeercast_re::pcp::{ChannelInfo, GnuId, TrackInfo};
use peercast_root::{ExitCode, IndexInfo, PortLevel, RestrictPortLevel, };
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use serde_with::{NoneAsEmptyString, serde_as};
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use tower_http::{cors::CorsLayer, set_header::SetResponseHeaderLayer};
use tracing::{debug, error, info, warn};

use bb8_redis::RedisConnectionManager;

use crate::{cli, db::{ConnectionPool, DatabaseConnection}, filter::filter_channels, portcheck::{get_portcheck_level, }, RootChannel, INDEX_TXT_FOOTER, REDIS_MASTER_KEY, REPOSITORY, _REDIS_MASTER_KEY};

struct ApiError(anyhow::Error);
// Tell axum how to convert `AppError` into a response.
impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for ApiError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

#[derive(Debug)]
struct ApiConfig {
    restrict_speed: u32,
    listener_hideable: bool,
    port_check_level: RestrictPortLevel,
    name_space: String,
}

#[derive(Debug, Clone)]
struct AppState(bb8::Pool<RedisConnectionManager>, Arc<ApiConfig>);

//-------------------------------------------------------------------------------
// Api Server
//-------------------------------------------------------------------------------
pub async fn server_http(
    args: cli::Args,
    listener: tokio::net::TcpListener,
    graceful_shutdown: CancellationToken,
) -> anyhow::Result<()> {
    use axum::routing::any;
    use tower_http::{
        services::ServeDir,
        trace::{DefaultMakeSpan, TraceLayer},
    };

    debug!("connecting to redis: {}", args.redis_url);
    let manager = RedisConnectionManager::new(args.redis_url).unwrap();
    let pool = bb8::Pool::builder().build(manager).await.unwrap();
    {
        // let mut conn = pool.get().await.unwrap();
        let mut conn = match tokio::time::timeout(Duration::from_millis(2000), pool.get()).await {
            Ok(Ok(conn)) => conn,
            Ok(Err(e)) => {
                error!("redis connect failed :{}", e);
                std::process::exit(ExitCode::Failure as i32);
            }
            Err(e) => {
                error!("redis connect timeout: {}", e);
                std::process::exit(ExitCode::Failure as i32);
            }
        };

        let key = format!("{}:CHECK", REDIS_MASTER_KEY());
        // conn.set::<&str, &str, ()>(&key, "CHECK_ME").await;
        match timeout(
            Duration::from_millis(2000),
            conn.set::<&str, &str, ()>(&key, "CHECK_ME"),
        )
        .await {
            Ok(Ok(())) => {
                debug!("redis connect SET COMMAND success");
            }
            Ok(Err(e)) => {
                error!("redis connect SET COMMAND failed: {}", e);
                std::process::exit(ExitCode::Failure as i32);
            }
            Err(_) => {
                error!("redis connect SET COMMAND timeout");
                std::process::exit(ExitCode::Failure as i32);
            }
        };

        match timeout(Duration::from_millis(1000), conn.get::<_, String>(&key)).await  {
            Ok(Ok(r))  => {
                debug!("redis connect GET COMMAND success: {}", r);
                assert_eq!(r, "CHECK_ME");
            }
            Ok(Err(e)) => {
                error!("redis connect GET COMMAND failed: {}", e);
                std::process::exit(ExitCode::Failure as i32);
            }
            Err(_) => {
                error!("redis connect GET COMMAND timeout");
                std::process::exit(ExitCode::Failure as i32);
            }
        };
    }
    tracing::debug!("successfully connected to redis and pinged it");

    let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    info!("asset_dir: {:?}", &assets_dir);

    let cor_origins: Vec<_> = args
        .allow_cors
        .iter()
        .map(|origin| origin.parse::<HeaderValue>().unwrap())
        .collect();
    info!("cor_origins: {:?}", &cor_origins);

    let cache_control_value = format!("max-age={}, public, mustrelvalidate", &args.cache_max_age);
    info!("cache-control: {}", &cache_control_value);

    let api_config = ApiConfig {
        restrict_speed: args.yp_limit_speed,
        listener_hideable: args.yp_listerer_hideable,
        port_check_level: args.yp_restrict_port_level,
        name_space: args.yp_name_space,
    };

    let tracker = tokio_util::task::TaskTracker::new();
    info!("START HTTP SERVER");

    let app = Router::new()
        .fallback_service(ServeDir::new(assets_dir).append_index_html_on_directories(true))
        .route("/index.txt", routing::get(index_txt))
        .route("/api/index.json", routing::get(index_json))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(cor_origins)
                .allow_methods([Method::GET]),
        )
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::CACHE_CONTROL,
            HeaderValue::from_bytes(cache_control_value.as_bytes()).unwrap(),
        ))
        .layer(args.ip_source.into_extension())
        .with_state(AppState(pool, Arc::new(api_config)));

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal(graceful_shutdown))
    .await
    .unwrap();

    Ok(())
}

fn shutdown_signal(graceful_shutdown: CancellationToken) -> BoxFuture<'static, ()> {
    async move {
        //
        graceful_shutdown.cancelled().await;
        info!("HTTP start graceful shutdown");
    }
    .boxed()
}

//-------------------------------------------------------------------------------
// Api Handlers
//-------------------------------------------------------------------------------
async fn index_txt(
    client_ip: ClientIp,
    query_params: Query<IndexTextParams>,
    mut conn: DatabaseConnection,
    state_config: State<Arc<ApiConfig>>,
) -> Result<String, ApiError> {
    let channels = index_json(client_ip, query_params, conn, state_config).await?;
    let channels: Vec<String> = channels.iter().map(|c| c.to_line_of_index_txt()).collect();

    Ok(itertools::join(channels, "\n"))
}

#[inline]
async fn index_json(
    ClientIp(ip): ClientIp,
    Query(params): Query<IndexTextParams>,
    mut conn: DatabaseConnection,
    State(config): State<Arc<ApiConfig>>,
) -> Result<Json<Vec<JsonChannel>>, ApiError> {
    let own_level = if let Some(host) = &params.Host {
        get_portcheck_level(&mut conn, ip, host.1).await.unwrap_or_else(|e| {
            error!("Failed to get_portcheck_level for {}:{} -> {}", host.0, host.1, e);
            PortLevel::None
        })
    } else {
        PortLevel::None
    };

    let mut channels: Vec<JsonChannel> = REPOSITORY().map_collect(|(id, ch)| ch.into());

    let mut channels: Vec<JsonChannel> = filter_channels(
        &config.name_space,
        config.listener_hideable,
        config.port_check_level,
        config.restrict_speed,
        own_level,
        0,
        channels,
    );

    // Footerを追加する
    channels.reserve(INDEX_TXT_FOOTER().len());
    channels.extend(INDEX_TXT_FOOTER().clone().into_iter().map(|e| e.into()));

    Ok(channels.into())
}

fn merged_channels() -> Vec<JsonChannel> {
    let mut channels: Vec<JsonChannel> = REPOSITORY().map_collect(|(id, ch)| ch.into());

    channels.reserve(INDEX_TXT_FOOTER().len());
    channels.extend(INDEX_TXT_FOOTER().clone().into_iter().map(|e| e.into()));
    channels
}


//-------------------------------------------------------------------------------
// ApiConfig Mapper
//-------------------------------------------------------------------------------
// AppStateからApiConfigを取り出すためのFromRef実装
impl FromRef<AppState> for Arc<ApiConfig> {
    fn from_ref(state: &AppState) -> Arc<ApiConfig> {
        state.1.clone()
    }
}

//-------------------------------------------------------------------------------
// Database mapper
//-------------------------------------------------------------------------------
impl FromRequestParts<AppState> for DatabaseConnection {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        _parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let pool = ConnectionPool::from_ref(&state.0);

        // let conn = pool.get_owned().await.map_err(internal_error)?;
        let ret_conn = timeout(Duration::from_millis(2000), pool.get_owned())
            .await
            .map_err(internal_error)?;
        let conn = ret_conn.map_err(internal_error)?;

        Ok(Self(conn))
    }
}

/// Utility function for mapping any error into a `500 Internal Server Error`
/// response.
fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

//-------------------------------------------------------------------------------
// Header/Query Mapper
//-------------------------------------------------------------------------------
/// index.txtに対するクエリ型
#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct IndexTextParams {
    #[serde(default, deserialize_with = "empty_string_as_none", alias = "host")]
    pub Host: Option<(String, u16)>,
}

fn empty_string_as_none<'de, D>(de: D) -> Result<Option<(String, u16)>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(de)?;
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => {
            let (host, port_str) = s
                .rsplit_once(':')
                .ok_or_else(|| serde::de::Error::custom("SplitFailed"))?;
            let port = port_str.parse::<u16>().map_err(serde::de::Error::custom)?;
            Ok(Some((host.into(), port)))
        }
    }
}

//-------------------------------------------------------------------------------
// Response structs
//-------------------------------------------------------------------------------
#[derive(Debug, Clone, Serialize)]
pub struct JsonChannel {
    pub id: GnuId,
    pub name: String,
    pub tracker_addr: Option<SocketAddr>,
    pub contact_url: String,
    pub genre: String,
    pub raw_genre: String, // namespace, listener_hideable, PortLimitを含むgenre
    pub desc: String,
    pub comment: String,
    /// MIME
    pub stream_type: String,
    /// 拡張子
    pub stream_ext: String,
    pub bitrate: i32,
    // filetype: String,
    // status: String,
    pub number_of_listener: i32,
    pub number_of_relay: i32,
    pub created_at: DateTime<Utc>, // FIX: 外部のCDNなどとの兼ね合いで配信時間が00:00意外になる可能性あり
    pub track: JsonTrack,

    #[serde(rename = "type")]
    pub typee: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonTrack {
    pub title: String,
    pub creator: String,
    pub url: String,
    pub album: String,
    pub genre: String,
}

impl From<&RootChannel> for JsonChannel {
    fn from(ch: &RootChannel) -> Self {
        let ChannelInfo {
            typ,
            name,
            genre,
            desc,
            comment,
            url,
            stream_type,
            stream_ext,
            bitrate,
        } = ch.channel_info();

        JsonChannel {
            id: ch.id(),
            name,
            tracker_addr: ch.tracker_addr(),
            contact_url: url,
            genre: genre.clone(),
            raw_genre: genre,
            desc,
            comment,
            typee: typ,
            stream_type,
            stream_ext,
            bitrate,
            number_of_listener: ch.number_of_listener(),
            number_of_relay: ch.number_of_relay(),
            created_at: ch.created_at(),
            track: ch.track_info().into(),
        }
    }
}

impl From<TrackInfo> for JsonTrack {
    fn from(t: TrackInfo) -> Self {
        JsonTrack {
            title: t.title,
            creator: t.creator,
            url: t.url,
            album: t.album,
            genre: t.genre,
        }
    }
}

impl JsonChannel {
    fn to_line_of_index_txt(&self) -> String {
        create_index_line(
            &self.name,
            &self.id,
            &self.tracker_addr,
            &self.contact_url,
            &self.genre,
            &self.desc,
            &self.comment,
            self.number_of_listener,
            self.number_of_relay,
            self.bitrate,
            &self.typee,
            &self.stream_type,
            &self.stream_ext,
            &self.created_at,
        )
    }
    pub fn empty() -> Self {
        // println!("DATETIME              {}", Utc.timestamp_opt(0, 0).unwrap());
        Self {
            id: GnuId::NONE,
            name: "".into(),
            tracker_addr: None,
            contact_url: "".into(),
            genre: "".into(),
            raw_genre: "".into(),
            desc: "".into(),
            comment: "".into(),
            typee: "".into(),
            stream_type: "".into(),
            stream_ext: "".into(),
            bitrate: 0,
            number_of_listener: 0,
            number_of_relay: 0,
            created_at: Utc.timestamp_opt(0, 0).unwrap(),
            track: JsonTrack {
                title: "".into(),
                creator: "".into(),
                url: "".into(),
                album: "".into(),
                genre: "".into(),
            },
        }
    }
}

fn create_index_line(
    name: &String,
    id: &GnuId,
    tracker_addr: &Option<SocketAddr>,
    contact_url: &String,
    genre: &String,
    desc: &String,
    comment: &String,
    number_of_listener: i32,
    number_of_relay: i32,
    bitrate: i32,
    typee: &String,
    stream_type: &String,
    stream_ext: &String,
    created_at: &DateTime<Utc>,
) -> String {
    use html_escape::{encode_quoted_attribute, encode_safe};
    let diff_time = Utc::now() - created_at;
    let hour = diff_time.num_hours();
    let min = diff_time.num_minutes() % 60;

    let addr = tracker_addr
        .as_ref()
        .map(|a| a.to_string())
        .unwrap_or_default();

    format!(
        "{name}<>{id}<>{addr}<>{contact_url}<>{genre}<>{desc}<>{number_of_listener}<>{number_of_relay}<>{bitrate}<>{typee}<><><><><>{name_escaped}<>{time_hour}:{time_min:02}<>click<>{comment}<>0",
        name = encode_safe(&name.clone()),
        id = id,
        addr = addr,
        contact_url = encode_quoted_attribute(&contact_url),
        genre = encode_safe(&genre),
        desc = encode_safe(&desc),
        number_of_listener = number_of_listener,
        number_of_relay = number_of_relay,
        bitrate = bitrate,
        typee = encode_safe(&typee),
        name_escaped = encode_safe(&name),
        time_hour = hour,
        time_min = min,
        comment = comment
    )
}

impl From<IndexInfo> for JsonChannel {
    fn from(value: IndexInfo) -> Self {
        let mut j = JsonChannel::empty();
        let IndexInfo {
            id,
            name,
            tracker_addr,
            contact_url,
            genre,
            desc,
            comment,
            typee,
            stream_type,
            stream_ext,
            bitrate,
            number_of_listener,
            number_of_relay,
            created_at,
        } = value;
        j.id = id;
        j.typee = typee;
        j.name = name;
        j.tracker_addr = tracker_addr;
        j.contact_url = contact_url;
        j.genre = genre.clone();
        j.raw_genre = genre;
        j.desc = desc;
        j.comment = comment;
        j.stream_ext = stream_ext;
        j.bitrate = bitrate;
        j.number_of_listener = number_of_listener;
        j.number_of_relay = number_of_relay;
        j.created_at = created_at.unwrap_or_else(|| chrono::Utc::now());
        j
    }
}
