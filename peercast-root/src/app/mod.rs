use std::sync::Arc;

use bb8_redis::RedisConnectionManager;
use libpeercast_re::pcp::PcpConnectionFactory;
use peercast_root::{IndexInfo, RestrictPortLevel, channel::RootChannel, repository::ChannelRepository};

pub mod cli;
pub mod handler;
pub mod logging;
pub mod portcheck;
pub mod server_http;
pub mod server_peercast;

#[derive(Debug)]
pub struct ApiConfig {
    pub restrict_speed: u32,
    pub listener_hideable: bool,
    pub port_check_level: RestrictPortLevel,
    pub name_space: String,
}

#[derive(Debug)]
pub struct AppState {
    pub config: Arc<ApiConfig>,
    pub index_txt_footer: Vec<IndexInfo>,
    pub redis_master_key: String,
    pub db_pool: bb8::Pool<RedisConnectionManager>,
    pub repository: ChannelRepository<RootChannel>,
    pub conn_factory: PcpConnectionFactory,
}

#[derive(Debug)]
pub struct ArcState(pub Arc<AppState>);

impl Clone for ArcState {
    fn clone(&self) -> Self {
        ArcState(Arc::clone(&self.0))
    }
}
