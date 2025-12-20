#![allow(dead_code)]
use std::{net::{IpAddr, Ipv4Addr}, path::PathBuf};

use anyhow::{Context, Ok};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::cli;

const DEFAULT_IPC_PATH: &str = "/tmp/peercast-re.sock";
const DEFAULT_SERVER_BIND : IpAddr = IpAddr::V4(Ipv4Addr::new(0,0,0,0));
const DEFAULT_SERVER_PORT : u16 = 17144;
const DEFAULT_API_BIND : IpAddr = IpAddr::V4(Ipv4Addr::new(127,0,0,1));
const DEFAULT_API_PORT : u16 = 17145;
const CONFIG_NAME: &str = "Settings.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub ipc_path: String,
    pub server_address: IpAddr,
    pub server_port: u16,
    pub api_address: IpAddr,
    pub api_port: u16,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            ipc_path: DEFAULT_IPC_PATH.into(),
            server_address: DEFAULT_SERVER_BIND,
            server_port: DEFAULT_SERVER_PORT,
            api_address: DEFAULT_API_BIND,
            api_port: DEFAULT_API_PORT,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum ConfigAddress {
    NoConfig(IpAddr),
    Config(IpAddr),
}

pub fn load_config(args: cli::Args) -> anyhow::Result<(Config, PathBuf)> {
    let exe_dir = std::env::current_exe()
        .context("Failed to get current exec path")?
        .parent()
        .context("Failed to get current exec parent path")?
        .to_path_buf();
    let bin_config_path = exe_dir.join(CONFIG_NAME);

    let config_dir = dirs::config_dir()
        .context("Failed to get config dir")?
        .join("peercast-re");
    let default_config_path = config_dir.join(CONFIG_NAME);

    // 次の順番で設定ファイルを読み込む
    // 1. コマンドライン引数での指定先
    // 2. 環境変数
    // 3. 実行ファイルのあるディレクトリの設定(<exe_dir>/Settings.toml)
    // 4. デフォルト設定(~/.config/peercast-re/Settings.toml)
    let config_path = if let Some(path) = std::env::var("PEERCAST_RE_CONFIG_FILE").ok() {
        // 1.
        debug!(
            "Loading configuration from env var PEERCAST_RE_CONFIG_FILE: {}",
            path
        );
        PathBuf::from(path)
    } else if let Some(path) = &args.config_file {
        // 2
        debug!("Loading configuration from command line arg: {:?}", path);
        path.clone()
    } else if bin_config_path.exists() {
        // 3
        debug!(
            "Loading configuration from binary path: {:?}",
            bin_config_path
        );
        bin_config_path
    } else {
        // 4
        debug!(
            "Loading configuration from default config path: {:?}",
            default_config_path
        );
        if !default_config_path.exists() {
            debug!("Default config file does not exist, using default settings.");
            std::fs::create_dir_all(config_dir).context("Failed to craete config directory")?;
            std::fs::File::create(&default_config_path).context("Failed to create config file")?;
            save_toml(&default_config_path, &Config::default())?;
        };
        default_config_path
    };

    info!("Using configuration file: {:?}", config_path);
    let config = load_toml(&config_path)?.merge_cli_args(args);

    Ok((config, config_path))
}

fn load_toml(path: &PathBuf) -> anyhow::Result<Config> {
    let toml_str = std::fs::read_to_string(path).context("Failed to read settings file")?;
    let config: Config = toml::from_str(&toml_str).context("Failed to parse settings from TOML")?;
    debug!("Loaded config from {:?}", path);
    debug!("Loaded config : {:?}", &config);
    Ok(config)
}

fn save_toml(path: &PathBuf, config: &Config) -> anyhow::Result<()> {
    let toml_str =
        toml::to_string_pretty(config).context("Failed to serialize settings to TOML")?;
    std::fs::write(path, toml_str).context("Failed to write settings to file")?;
    debug!("Saved config to {:?}", path);
    debug!("Saved config : {:?}", &config);
    Ok(())
}

impl Config {
    fn merge_cli_args(mut self, args: cli::Args) -> Self {
        if let Some(addr) = args.server_address {
            self.server_address = addr;
        }
        if let Some(port) = args.server_port {
            self.server_port = port;
        }
        if let Some(addr) = args.api_address {
            self.api_address = addr;
        }
        if let Some(port) = args.api_port {
            self.api_port = port;
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::cli::Args;

    use super::*;

    #[test]
    fn test_cli_args_source() {
        let args = Args {
            config_file: Some(PathBuf::from("test_config.toml")),
            ipc_path: Some(PathBuf::from("/tmp/peercast-re.sock")),
            server_address: Some("127.0.0.127".parse().unwrap()),
            server_port: None,
            api_address: None,
            api_port: Some(18000),
            command: None,
        };

        let config = Config::default();
        assert_eq!(config.server_address, DEFAULT_SERVER_BIND);
        assert_eq!(config.server_port, DEFAULT_SERVER_PORT);
        assert_eq!(config.api_address, DEFAULT_API_BIND);
        assert_eq!(config.api_port, DEFAULT_API_PORT);

        let config = config.merge_cli_args(args);
        assert_eq!(
            config.server_address,
            "127.0.0.127".parse::<IpAddr>().unwrap()
        );
        assert_eq!(config.server_port, DEFAULT_SERVER_PORT);
        assert_eq!(config.api_address, DEFAULT_API_BIND);
        assert_eq!(config.api_port, 18000);
    }
}
