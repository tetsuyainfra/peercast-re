use std::path::PathBuf;

use clap::{Parser, command};
use tower_http::follow_redirect::policy::PolicyExt;

////////////////////////////////////////////////////////////////////////////////
/// Parse args
///
#[derive(Clone, Debug, Parser)]
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
        short = 'b', long = "bind",
        value_name = "IP_ADDRESS",
        env = "PEERCAST_RE_BIND",
        // default_value = "0.0.0.0"
    )]
    pub server_address: Option<std::net::IpAddr>,

    #[clap(
        short='p', long="port",
        value_name = "PORT",
        env = "PEERCAST_RE_PORT",
        //  default_value = "17144",
        value_parser = clap::value_parser!(u16).range(5000..)
    )]
    pub server_port: Option<u16>,

    #[clap(
        long = "api-bind",
        value_name = "API_IP_ADDRESS",
        env = "PEERCAST_RE_API_BIND",
        // default_value = "0.0.0.0"
    )]
    pub api_address: Option<std::net::IpAddr>,

    #[clap(
        long="api-port",
        value_name = "API_PORT",
        env = "PEERCAST_RE_API_PORT",
        //  default_value = "17145",
        value_parser = clap::value_parser!(u16).range(5000..)
    )]
    pub api_port: Option<u16>,
}
