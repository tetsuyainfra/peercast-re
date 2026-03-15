use std::sync::Arc;

use libpeercast_re::pcp::GnuId;
use peercast_root::{
    connection::{RootConnectionFactory, RootConnectionManager},
    model::IndexInfo,
    repository::RootRepository2,
    service::YellowPageService,
};

pub mod cli;
pub mod http;
pub mod logging;
pub mod peercast;
pub mod portcheck;

pub use http::server_http;
pub use peercast::server_peercast;

#[allow(dead_code)]
#[derive(Debug)]
pub struct ApiConfig {
    /// CORS許可リスト
    pub allow_cors: Vec<String>,

    /// APIが返すデータのキャッシュの最大有効期限（秒）
    pub cache_max_age: u32,

    /// クライアントIPの取得元
    pub client_ip_source: cli::ClientIpSourceArg,
}

#[allow(non_snake_case)]
#[derive(Debug, serde::Serialize)]
pub struct EmbedTemplateCtx {
    pub EMBED_TITLE: String,
    pub EMBED_YP_NAME: String,
    pub EMBED_URL_HTTP: String,
    pub EMBED_URL_PCP: String,

    /// トラッカーのPeerCastでジャンルに含ませるべき文字列
    pub tracker_yp_name: String,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct AppState {
    pub self_session_id: GnuId,
    pub config: Arc<ApiConfig>,
    pub index_txt_footer: Vec<IndexInfo>,
    pub db_pool: sqlx::Pool<sqlx::sqlite::Sqlite>,
    pub yellow_page: Arc<YellowPageService>,
    pub repository: RootRepository2,
    //
    pub embed_tmpl_ctx: EmbedTemplateCtx,
    //
    pub connection_factory: RootConnectionFactory,
    pub connection_manager: RootConnectionManager,
}

#[derive(Debug)]
pub struct ArcState(pub Arc<AppState>);

impl Clone for ArcState {
    fn clone(&self) -> Self {
        ArcState(Arc::clone(&self.0))
    }
}

impl std::ops::Deref for ArcState {
    type Target = AppState;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
