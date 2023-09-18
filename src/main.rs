use clap::Parser;
use std::path::PathBuf;

use peercast_re::{
    app::cui::{self, CuiError},
    config::{Config, ConfigLoader, ConfigPath},
    error::ConfigError,
};

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
struct Cli {
    #[clap(
        short = 'b',
        long = "bind",
        value_name = "IP_ADDRESS",
        default_value = "0.0.0.0"
    )]
    server_address: Option<std::net::IpAddr>,

    #[clap(short='p', long="port", value_name = "PORT", default_value = "17144",
                    value_parser = clap::value_parser!(u16).range(5000..))]
    server_port: Option<u16>,
}

impl Cli {
    /// merge Config and Cli instance.
    fn merge_with(self, config: &Config) -> Config {
        use peercast_re::config::ConfigAddress;

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

// Cliクラスで引数を受け取って処理しても良いのではないか
// default_value, dafault_environmentってあるっぽい
// #[clap(
//     short = 'c',
//     long = "config",
//     value_name = "CONFIG_FILE",
//     env = "PEERCAST_RT_CONFIG"
// )]
// config_file: Option<PathBuf>,
fn load_config() -> Result<(PathBuf, Config), ConfigError> {
    fn env_str() -> &'static str {
        if cfg!(debug_assersion) {
            "DEBUG_PEERCAST_RT_CONFIG"
        } else {
            "PEERCAST_RT_CONFIG"
        }
    }

    let (path, config) = ConfigLoader::<Config>::new()
        .add_source(ConfigPath::Env(env_str().into()))
        .add_source(ConfigPath::Path("peercast-rt.ini".into()))
        .default_source(ConfigPath::PathBuf(
            dirs::config_dir().unwrap().join("peercast/peercast-rt.ini"),
        )) // これでいいのか？
        .load();

    Ok((path, config?))
}

/// initialize logging
fn logging_init() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};
    // tracing_subscriber::fmt()
    //     // enable everything
    //     .with_max_level(tracing::Level::TRACE)
    //     // display source code file paths
    //     .with_file(true)
    //     // display source code line numbers
    //     .with_line_number(true)
    //     // disable targets
    //     .with_target(false)
    //     // sets this to be the default, global collector for this application.
    //     .init();

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_file(true)
                .with_line_number(true)
                .with_target(false),
        )
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            println!("RUST_LOG=debug");
            "debug".into()
        }))
        .init();
}

////////////////////////////////////////////////////////////////////////////////
// MAIN
//
fn main() {
    logging_init();
    let cli = Cli::parse();
    println!("{:#?}", &cli);

    let Ok((config_path, config)) = load_config() else {
        std::process::exit(exitcode::CONFIG);
    };
    let config = cli.merge_with(&config);

    match cui::CuiApp::run(config_path, config) {
        Ok(_) => std::process::exit(exitcode::OK),
        Err(e) => {
            println!("{e}");
            match e {
                CuiError::LoadConfiguration => std::process::exit(exitcode::CONFIG),
                CuiError::ApplicationError => std::process::exit(exitcode::SOFTWARE),
                CuiError::ShutdownFailed(_) => std::process::exit(exitcode::SOFTWARE),
                CuiError::Io(_) => std::process::exit(exitcode::IOERR),
            }
        }
    }
}
