use std::net::IpAddr;

use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};

use axum_client_ip::ClientIp;
use hyper::StatusCode;
// use futures_util::FutureExt;
use libpeercast_re::repository::Repository;
use peercast_root::{model::ChannelMeta, service::HostCheckService};
use serde::Deserialize;

use crate::app::ArcState;

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
    State(state): State<ArcState>,
) -> Result<String, ApiError> {
    let ip_addr: IpAddr = client_ip.to_canonical();

    // Hostヘッダの有無で処理を分岐
    // Hostヘッダがある場合はそちらを優先する。ただし接続元IPアドレスはハンドラーで取得したものを使う。
    let (target_ip, target_port) = match params.Host {
        Some((_host, port)) => (ip_addr, port),
        None => (ip_addr, 7144),
    };

    let _x = HostCheckService::do_host_check(&state.connection_factory, &state.db_pool, target_ip, target_port).await?;

    let channels: Vec<ChannelMeta> = state.repository.map_collect(|_id, ch| ch.channel_meta());
    // let channels = state.yellow_page.filter_channel_meta(channels);
    let string_channels: Vec<String> = channels.iter().map(|c| c.to_line_of_index_txt()).collect();

    Ok(itertools::join(string_channels, "\n"))
}

pub async fn index_json(
    ClientIp(_client_ip): ClientIp,
    Query(_params): Query<IndexTextParams>,
    state: State<ArcState>,
) -> Result<Json<Vec<ChannelMeta>>, ApiError> {
    let channels: Vec<ChannelMeta> = state.repository.map_collect(|_id, ch| ch.channel_meta());
    // let channels = state.yellow_page.filter_channel_meta(channels);

    Ok(Json(channels))
}

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

/// Hostクエリの分解に使う補助メソッド 0字の文字列をNoneとして扱う
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

//-------------------------------------------------------------------------------
// Static Files Handlers
//-------------------------------------------------------------------------------
#[cfg(not(debug_assertions))]
#[derive(rust_embed::RustEmbed)]
#[folder = "src/public/"]
struct Assets;

pub fn static_router() -> axum::Router {
    #[cfg(debug_assertions)]
    {
        // 開発時：ローカルディレクトリをそのまま配信
        axum::Router::new().fallback_service(axum::routing::get_service(
            tower_http::services::ServeDir::new("src/public").append_index_html_on_directories(true),
        ))
    }

    #[cfg(not(debug_assertions))]
    {
        debug!("Assets include files");
        for a in Assets::iter() {
            debug!("- {}", a.as_ref());
        }
        // リリース時：バイナリ埋め込み
        axum::Router::new()
            // .route("/", axum::routing::get(embed_handler))
            // .route("/{*path}", axum::routing::get(embed_handler))
            .fallback(routing::get(embed_handler))
    }
}

#[cfg(not(debug_assertions))]
async fn embed_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path();
    trace!("REQLINE: {}", path);
    let path = if path.ends_with("/") {
        [path, "index.html"].concat()
    } else {
        path.into()
    };
    let path = if path.starts_with("/") {
        path.replacen("/", "", 1)
    } else {
        path
    };
    trace!("   PATH: {}", path);

    match Assets::get(&path) {
        Some(content) => {
            let body = content.data.into_owned();
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            (
                //
                [(header::CONTENT_TYPE, mime.as_ref())],
                body,
            )
                .into_response()
        }
        None => {
            (
                //
                StatusCode::NOT_FOUND,
                "Not Found",
            )
                .into_response()
        }
    }
}
