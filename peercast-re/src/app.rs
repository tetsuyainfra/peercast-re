use crate::config::Config;

#[derive(Debug, Clone, Default)]
pub struct Store {
    #[allow(dead_code)]
    pub config: Config, // #[allow(dead_code)]
    pub config_path: std::path::PathBuf,
}
