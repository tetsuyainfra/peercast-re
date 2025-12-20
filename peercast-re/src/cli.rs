use std::path::PathBuf;

use clap::{Parser, command};

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
        short = 'I', long = "ipc-bind",
        value_name = "IPC_PATH",
        env = "PEERCAST_RE_IPC_PATH",
        default_value = "/tmp/peercast-re.sock"
    )]
    pub ipc_path: Option<PathBuf>,

    #[clap(
        short = 'B', long = "bind",
        value_name = "IP_ADDRESS",
        env = "PEERCAST_RE_BIND",
        // default_value = "0.0.0.0"
    )]
    pub server_address: Option<std::net::IpAddr>,

    #[clap(
        short='P', long="port",
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



    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Clone, Debug, clap::Subcommand)]
pub enum Commands {
    Server {},
    Listen {
        #[clap(value_name = "LISTEN_URL")]
        url: url::Url,
    },
}



#[ cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use url::Url;

    use super::*;

    #[test]
    fn test_args_parse() {
        let args = Args::parse_from(&[
            "peercast-re",
            "-C", "config.toml",
            "-B", "10.10.10.10"
        ]);
        assert_eq!(args.config_file.unwrap(), PathBuf::from("config.toml"));
        assert_eq!(args.server_address.unwrap(), IpAddr::V4(Ipv4Addr::new(10,10,10,10)));
        assert_eq!(args.server_port, None);
    }

    #[test]
    fn test_args_commands() {
        let args = Args::parse_from(&[
            "peercast-re",
            "listen",
            "http://example.com/stream",
        ]);
        match args.command.unwrap() {
            Commands::Listen { url } => {
                assert_eq!(url, Url::parse("http://example.com/stream").unwrap());
            }
            _ => unreachable!("")
        }
    }
}
