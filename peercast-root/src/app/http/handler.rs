
use axum::{
    Json,
    extract::{FromRef, FromRequestParts, Query, State},
    response::IntoResponse,
};

use axum_client_ip::ClientIp;
use futures_util::FutureExt;
use hyper::StatusCode;
use libpeercast_re::repository::Repository;
use peercast_root::{
    PortLevel, model::JsonChannel, filter::filter_channels
};
use serde::Deserialize;
use sqlx::{Pool, Sqlite};
use tokio::time::timeout;


use crate::app::{ArcState};

pub struct ApiError(anyhow::Error);
// Tell axum how to convert `AppError` into a response.
impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Something went wrong: {}", self.0)).into_response()
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

/*
#[derive(Debug)]
pub struct ApiConfig {
    pub restrict_speed: u32,
    pub listener_hideable: bool,
    pub port_check_level: RestrictPortLevel,
    pub name_space: String,
}

#[derive(Debug, Clone)]
pub struct AppState(pub bb8::Pool<RedisConnectionManager>, pub Arc<ApiConfig>);

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
        match timeout(Duration::from_millis(2000), conn.set::<&str, &str, ()>(&key, "CHECK_ME")).await {
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

        match timeout(Duration::from_millis(1000), conn.get::<_, String>(&key)).await {
            Ok(Ok(r)) => {
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

    let cor_origins: Vec<_> = args.allow_cors.iter().map(|origin| origin.parse::<HeaderValue>().unwrap()).collect();
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
        .layer(TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::default().include_headers(true)))
        .layer(CorsLayer::new().allow_origin(cor_origins).allow_methods([Method::GET]))
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::CACHE_CONTROL,
            HeaderValue::from_bytes(cache_control_value.as_bytes()).unwrap(),
        ))
        .layer(args.ip_source.into_extension())
        .with_state(AppState(pool, Arc::new(api_config)));

    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
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
    */

//-------------------------------------------------------------------------------
// Api Handlers
//-------------------------------------------------------------------------------
pub async fn index_txt(
    client_ip: ClientIp,
    query_params: Query<IndexTextParams>,
    state: State<ArcState>,
) -> Result<String, ApiError> {
    // let channels = index_json(client_ip, query_params, conn, state).await?;
    // let channels: Vec<String> = channels.iter().map(|c| c.to_line_of_index_txt()).collect();

    // Ok(itertools::join(channels, "\n"))
    Ok(String::new())
}

pub async fn index_json(
    ClientIp(ip): ClientIp,
    Query(params): Query<IndexTextParams>,
    State(state): State<ArcState>,
) -> Result<Json<Vec<JsonChannel>>, ApiError> {
    let port = params.Host.as_ref().map(|(_host, port)| *port);
    let own_level = check_host_port_level(&state.db_pool, ip, port).await;

    let channels: Vec<JsonChannel> = state.0.repository2.map_collect(|_id, ch|  ch.into());
    let config = &state.0.config;

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
    channels.reserve(state.0.index_txt_footer.len());
    channels.extend(state.0.index_txt_footer.iter().map(|e| e.into()));

    Ok(channels.into())
}

async fn check_host_port_level(db: &Pool<Sqlite>, ip: std::net::IpAddr, port: Option<u16>) -> PortLevel {
    // HostCheckService::do_host_check().await.unwrap_or(PortLevel::None)
    PortLevel::None
}

// fn merged_channels(state: ArcState) -> Vec<JsonChannel> {
//     let mut channels: Vec<JsonChannel> = state.0.repository.map_collect(|(id, ch)| ch.into());

//     channels.reserve(INDEX_TXT_FOOTER().len());
//     channels.extend(INDEX_TXT_FOOTER().clone().into_iter().map(|e| e.into()));
//     channels
// }

//-------------------------------------------------------------------------------
// ApiConfig Mapper
//-------------------------------------------------------------------------------
// AppStateからApiConfigを取り出すためのFromRef実装
// impl FromRef<Arc<AppState>> for Arc<ApiConfig> {
//     fn from_ref(state: &Arc<AppState>) -> Arc<ApiConfig> {
//         state.config
//     }
// }

//-------------------------------------------------------------------------------
// Database mapper
//-------------------------------------------------------------------------------
// impl FromRequestParts<ArcState> for DatabaseConnection {
//     type Rejection = (StatusCode, String);

//     async fn from_request_parts(
//         _parts: &mut axum::http::request::Parts,
//         state: &ArcState,
//     ) -> Result<Self, Self::Rejection> {
//         let pool = ConnectionPool::from_ref(&state.0.db_pool);

//         // let conn = pool.get_owned().await.map_err(internal_error)?;
//         let ret_conn = timeout(Duration::from_millis(2000), pool.get_owned()).await.map_err(internal_error)?;
//         let conn = ret_conn.map_err(internal_error)?;

//         Ok(Self(conn))
//     }
// }

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
pub struct IndexTextParams {
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
            let (host, port_str) = s.rsplit_once(':').ok_or_else(|| serde::de::Error::custom("SplitFailed"))?;
            let port = port_str.parse::<u16>().map_err(serde::de::Error::custom)?;
            Ok(Some((host.into(), port)))
        }
    }
}
