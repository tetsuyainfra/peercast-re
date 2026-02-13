use std::net::{IpAddr, SocketAddr};

use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};

use axum_client_ip::ClientIp;
// use futures_util::FutureExt;
use hyper::StatusCode;
use libpeercast_re::repository::Repository;
use peercast_root::{PortLevel, model::JsonChannel, service::HostCheckService};
use serde::Deserialize;
use sqlx::{Pool, Sqlite};
use tracing_subscriber::layer::Filter;

use crate::app::{ArcState, yp::FilterConfig};

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

//-------------------------------------------------------------------------------
// Api Handlers
//-------------------------------------------------------------------------------
pub async fn index_txt(
    ClientIp(client_ip): ClientIp,
    Query(params): Query<IndexTextParams>,
    state: State<ArcState>,
) -> Result<String, ApiError> {
    let ip_addr: IpAddr = client_ip.to_canonical();

    // Hostヘッダの有無で処理を分岐
    // Hostヘッダがある場合はそちらを優先する。ただし接続元IPアドレスはハンドラーで取得したものを使う。
    let (target_ip, target_port) = match params.Host {
        Some((host, port)) => (ip_addr, port),
        None => (ip_addr, 7144),
    };

    let x = HostCheckService::do_host_check(&state.db_pool, target_ip, target_port).await?;

    let filter_config = FilterConfig {};
    let channels: Vec<JsonChannel> = state.repository.map_collect(|_id, ch| ch.into());
    let string_channels = state.yellow_page.to_index_txt(&filter_config, channels);

    Ok(itertools::join(string_channels, "\n"))
}

pub async fn index_json(
    ClientIp(client_ip): ClientIp,
    Query(_params): Query<IndexTextParams>,
    state: State<ArcState>,
) -> Result<Json<Vec<JsonChannel>>, ApiError> {
    let channels: Vec<JsonChannel> = state.repository.map_collect(|_id, ch| ch.into());
    let json_channels = state.yellow_page.to_index_json(&FilterConfig {}, channels);

    Ok(json_channels)
}

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

/// Utility function for mapping any error into a `500 Internal Server Error` response.
#[allow(dead_code)]
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
#[allow(non_snake_case, unused)]
#[derive(Debug, Deserialize)]
pub struct IndexTextParams {
    #[serde(default, deserialize_with = "empty_string_as_none", alias = "host")]
    #[allow(non_snake_case, unused)]
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
