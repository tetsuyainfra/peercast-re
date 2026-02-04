use std::sync::Arc;

use bb8_redis::RedisConnectionManager;
use peercast_root::{RestrictPortLevel, channel::RootChannel, repository::ChannelRepository};

pub mod cli;
pub mod handler;
pub mod logging;
pub mod portcheck;

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
    pub db_pool: bb8::Pool<RedisConnectionManager>,
    pub repository: ChannelRepository<RootChannel>,
}

#[derive(Debug)]
pub struct ArcState(pub Arc<AppState>);

impl Clone for ArcState {
    fn clone(&self) -> Self {
        ArcState(Arc::clone(&self.0))
    }
}
