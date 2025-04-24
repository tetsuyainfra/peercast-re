use std::{net::IpAddr, path::PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub config_path : Option<PathBuf>,
    pub server_address: ConfigAddress,
    pub server_port: u16,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            config_path: None,
            server_address: ConfigAddress::NoConfig("0.0.0.0".parse().unwrap()),
            server_port: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigAddress {
    NoConfig(IpAddr),
    Config(IpAddr),
}

#[derive(Debug, Error)]
pub enum ConfigError {}

pub fn load_config(env_or_args: Option<PathBuf>) -> Result<(PathBuf, Config), ConfigError> {
    let exe_dir = PathBuf::from(std::env::current_exe().unwrap().parent().unwrap());

    // let (path, config) = ConfigLoader::<Config>::new()
    //     .env_or_args(env_or_args)
    //     .add_source(exe_dir.join("peercast-re.ini"))
    //     .default_source(
    //         dirs::config_dir()
    //             .unwrap()
    //             .join("peercast-re/peercast-re.ini"),
    //     ) // これでいいのか？
    //     .load();

    let path = todo!();
    let config = todo!();

    debug!(?config);
    Ok((path, config))
}
