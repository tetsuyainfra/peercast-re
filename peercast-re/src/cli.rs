use std::path::PathBuf;

use clap::{Parser, command};

use crate::config::{Config, ConfigAddress};

////////////////////////////////////////////////////////////////////////////////
/// Parse args
///
#[derive(Debug, Parser)]
#[clap(
        name = env!("CARGO_PKG_NAME"),
        author = env!("CARGO_PKG_AUTHORS"),
        about = env!("CARGO_PKG_DESCRIPTION"),
    )]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Args {
    #[clap(
        short = 'C',
        long = "config",
        value_name = "CONFIG_FILE",
        env = "PEERCAST_RE_CONFIG"
    )]
    pub config_file: Option<PathBuf>,

    #[clap(
        short = 'b',
        long = "bind",
        value_name = "IP_ADDRESS",
        // default_value = "0.0.0.0"
    )]
    pub server_address: Option<std::net::IpAddr>,

    #[clap(
        short='p', long="port", value_name = "PORT",
        env = "PEERCAST_RE_PORT",
        //  default_value = "17144",
        value_parser = clap::value_parser!(u16).range(5000..)
    )]
    pub server_port: Option<u16>,
}

impl Args {
    /// merge Config and Cli instance.
    pub fn merge_with(self, config: &Config) -> Config {

        let mut config = config.clone();

        if let Some(ip) = self.server_address {
            config.server_address = ConfigAddress::NoConfig(ip)
        };
        if let Some(port) = self.server_port {
            config.server_port = port
        };
        config
    }
}
