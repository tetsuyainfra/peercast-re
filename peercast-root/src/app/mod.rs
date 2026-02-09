use std::sync::Arc;

use libpeercast_re::pcp::PcpConnectionFactory;
use peercast_root::{ IndexInfo, RestrictPortLevel, repository::RootRepository2};

pub mod cli;
pub mod logging;
pub mod portcheck;
pub mod http;
pub mod peercast;
pub mod yp;

pub use http::server_http;
pub use peercast::server_peercast;

#[derive(Debug)]
pub struct ApiConfig {
    /// CORS許可リスト
    pub allow_cors: Vec<String>,

    /// APIが返すデータのキャッシュの最大有効期限（秒）
    pub cache_max_age: u32,

    /// クライアントIPの取得元
    pub client_ip_source: axum_client_ip::ClientIpSource,

    /// PortCheckの制限速度
    pub restrict_speed: u32,
    pub listener_hideable: bool,
    pub port_check_level: RestrictPortLevel,
    pub name_space: String,
}

#[derive(Debug)]
pub struct AppState {
    pub config: Arc<ApiConfig>,
    pub index_txt_footer: Vec<IndexInfo>,
    pub db_pool: sqlx::Pool<sqlx::sqlite::Sqlite>,
    pub yellow_page: Arc<yp::YellowPage>,
    pub repository2: RootRepository2,
    pub conn_factory: PcpConnectionFactory,
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
