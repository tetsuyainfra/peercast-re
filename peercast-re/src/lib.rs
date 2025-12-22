pub mod channel;
pub mod cli;
pub mod config;
pub mod handler;
pub mod peercast;
pub mod repository;

pub mod prelude;

pub const SWAGGER_PATH: &str = "/swagger-ui";

#[derive(Debug, Clone, Default)]
pub struct Store {
    #[allow(dead_code)]
    pub config: crate::config::Config, // #[allow(dead_code)]
    pub config_path: std::path::PathBuf,
}

pub type AppState = std::sync::Arc<Store>;
