use tokio::sync::mpsc::UnboundedSender;

use crate::channel::ReChannel;

pub mod channel;
pub mod cli;
pub mod config;
pub mod handler;
pub mod peercast;
pub mod repository;

pub mod prelude;

pub const SWAGGER_PATH: &str = "/swagger-ui";

#[derive(Debug, Clone)]
pub struct State {
    #[allow(dead_code)]
    pub config: crate::config::Config, // #[allow(dead_code)]
    pub config_path: std::path::PathBuf,
    pub repository: repository::ReChannelRepository<ReChannel>,
    pub rtmp_manager_sender: UnboundedSender<libpeercast_re::rtmp::stream_manager::StreamManagerMessage>,
}

pub type AppState = std::sync::Arc<State>;

#[cfg(test)]
mod test_helper;
