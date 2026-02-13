use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};

use axum_client_ip::ClientIp;
// use futures_util::FutureExt;
use hyper::StatusCode;
use libpeercast_re::repository::Repository;
use peercast_root::{PortLevel, model::JsonChannel};
use serde::Deserialize;
use sqlx::{Pool, Sqlite};

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
    _client_ip: ClientIp,
    Query(_params): Query<IndexTextParams>,
    state: State<ArcState>,
) -> Result<String, ApiError> {
    let channels: Vec<JsonChannel> = state.0.repository2.map_collect(|_id, ch| ch.into());
    let string_channels = state.0.yellow_page.to_index_txt(&FilterConfig {}, channels);

    Ok(itertools::join(string_channels, "\n"))
}

pub async fn index_json(
    ClientIp(_client_ip): ClientIp,
    Query(_params): Query<IndexTextParams>,
    state: State<ArcState>,
) -> Result<Json<Vec<JsonChannel>>, ApiError> {
    // let port = params.Host.as_ref().map(|(_host, port)| *port);
    // let own_level = check_host_port_level(&state.db_pool, ip, port).await;

    let channels: Vec<JsonChannel> = state.0.repository2.map_collect(|_id, ch| ch.into());
    let json_channels = state.0.yellow_page.to_index_json(&FilterConfig {}, channels);

    Ok(json_channels)
}

#[allow(dead_code)]
async fn check_host_port_level(_db: &Pool<Sqlite>, _ip: std::net::IpAddr, _port: Option<u16>) -> PortLevel {
    // HostCheckService::do_host_check().await.unwrap_or(PortLevel::None)
    PortLevel::None
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
