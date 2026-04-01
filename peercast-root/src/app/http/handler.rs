use std::net::IpAddr;

use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};

use axum_client_ip::ClientIp;
use hyper::StatusCode;
use libpeercast_re::repository::Repository;
use minijinja::Environment;
use peercast_root::{
    db::SqliteCheckedHostRepository,
    model::ChannelMeta,
    service::{HostCheckService, PingPortChecker},
};
use serde::Deserialize;
use tracing::{debug, info};

use crate::app::{ArcState, EmbedTemplateCtx};

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
/// /index.txtを提供するハンドラー
pub async fn index_txt(
    ClientIp(client_ip): ClientIp,
    Query(params): Query<IndexTextParams>,
    State(state): State<ArcState>,
) -> Result<String, ApiError> {
    let channels = _get_index(client_ip, params.Host, state).await?;
    let string_channels: Vec<String> = channels.iter().map(|c| c.to_line_of_index_txt()).collect();

    Ok(itertools::join(string_channels, "\n"))
}

/// /index.jsonを提供するハンドラー
pub async fn index_json(
    ClientIp(client_ip): ClientIp,
    Query(params): Query<IndexTextParams>,
    State(state): State<ArcState>,
) -> Result<Json<Vec<ChannelMeta>>, ApiError> {
    let channels = _get_index(client_ip, params.Host, state).await?;
    Ok(Json(channels))
}

async fn _get_index(
    client_ip: IpAddr,
    query_host: Option<(String, u16)>,
    state: ArcState,
) -> Result<Vec<ChannelMeta>, ApiError> {
    // Hostヘッダの有無で処理を分岐
    // Hostヘッダがある場合はそちらを優先する。ただし接続元IPアドレスはハンドラーで取得したものを使う。
    let (target_ip, target_port) = match query_host {
        Some((_host, port)) => (client_ip, port),
        None => (client_ip, 7144),
    };

    let repo = SqliteCheckedHostRepository::new(state.db_pool.clone());
    let port_checker = PingPortChecker::new(&state.connection_factory);
    let checker = HostCheckService::new(repo, port_checker);

    let host = checker.do_check(target_ip, target_port).await?;

    let channels: Vec<ChannelMeta> = state.repository.map_collect(|_id, ch| ch.channel_meta());
    let channels = state.yellow_page.filter_channel_meta(&host, channels);

    Ok(channels)
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

#[cfg(not(debug_assertions))]
struct StaticState {
    env: Environment<'static>,
    template_ctx: EmbedTemplateCtx,
}

pub fn static_router(
    #[allow(unused)] assets_dir: Option<std::path::PathBuf>,
    #[allow(unused)] template_ctx: EmbedTemplateCtx,
) -> axum::Router {
    #[cfg(debug_assertions)]
    {
        // 開発時：ローカルディレクトリをそのまま配信
        axum::Router::new().fallback_service(axum::routing::get_service(
            tower_http::services::ServeDir::new("src/public").append_index_html_on_directories(true),
        ))
    }

    #[cfg(not(debug_assertions))]
    {
        use std::sync::Arc;
        if let Some(assets_dir) = assets_dir {
            info!("static router is ServeDir");
            // リリース時：指定ディレクトリを参照
            axum::Router::new().fallback_service(axum::routing::get_service(
                tower_http::services::ServeDir::new(assets_dir).append_index_html_on_directories(true),
            ))
        } else {
            info!("static router is Assets(Embed)");
            use std::sync::Arc;

            use axum::routing;
            debug!("Assets include files");
            for a in Assets::iter() {
                debug!("- {}", a.as_ref());
            }

            // Template Environment
            let mut env = minijinja::Environment::new();
            env.set_loader(|name| {
                match mime_guess::from_path(name).first() {
                    None => {
                        return Ok(None);
                    }
                    Some(m) => {
                        if m.type_() != mime_guess::mime::TEXT {
                            return Ok(None);
                        }
                    }
                };

                let Some(content) = Assets::get(&name) else {
                    return Ok(None);
                };

                let s = String::from_utf8_lossy(&content.data[..]).into_owned();
                Ok(Some(s))
            });
            let state = StaticState {
                env: env,
                template_ctx,
            };

            // リリース時：バイナリ埋め込み
            axum::Router::new()
                // .route("/", axum::routing::get(embed_handler))
                // .route("/{*path}", axum::routing::get(embed_handler))
                .fallback(routing::get(embed_handler))
                .with_state(Arc::new(state))
        }
    }
}

#[cfg(not(debug_assertions))]
async fn embed_handler(uri: hyper::Uri, State(state): State<Arc<StaticState>>) -> impl IntoResponse {
    let path = uri.path();
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

    let mime = mime_guess::from_path(&path).first_or_octet_stream();

    match state.env.get_template(&path) {
        // minijinjaで生成
        Ok(tmpl) => match tmpl.render(&state.template_ctx) {
            Ok(rendered) => {
                (
                    //
                    [(hyper::header::CONTENT_TYPE, mime.as_ref())],
                    rendered,
                )
                    .into_response()
            }
            Err(_e) => (StatusCode::INTERNAL_SERVER_ERROR, "Error").into_response(),
        },

        // 静的ファイルにフォールバック
        Err(_e) => {
            match Assets::get(&path) {
                Some(content) => {
                    let body = content.data.into_owned();
                    (
                        //
                        [(hyper::header::CONTENT_TYPE, mime.as_ref())],
                        body,
                    )
                        .into_response()
                }
                None => (StatusCode::NOT_FOUND, "Not Found").into_response(),
            }
        }
    }
}
