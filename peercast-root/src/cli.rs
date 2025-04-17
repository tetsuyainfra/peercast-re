use std::process::exit;

use clap::{Parser, Subcommand};

/// Simple Daemon Program
#[derive(Parser, Debug, Clone)]
#[command(name = env!("CARGO_BIN_NAME"))]
#[command(version, about, long_about = None)]
pub struct Args {
    /// PeerCast root server address
    #[arg(short, long, default_value = "0.0.0.0")]
    pub bind: std::net::IpAddr,

    #[cfg(not(debug_assertions))]
    /// PeerCast root server port
    #[arg(short, long, default_value_t = 7144)]
    pub port: u16,

    #[cfg(debug_assertions)]
    /// PeerCast root server port
    #[arg(short, long, default_value_t = 17144)]
    pub port: u16,

    /// HTTP API address
    #[arg(long, default_value = "127.0.0.1")]
    pub api_bind: std::net::IpAddr,

    /// HTTP API port
    #[arg(long, default_value_t = 7143)]
    pub api_port: u16,

    // TODO: TIMEZONEの実装
    // #[arg(long, default_value_t = 7143)]
    // pub timezone: u16,
    /// Enable daemon-mode
    #[arg(short = 'D', long, default_value_t = false)]
    pub daemon: bool,

    /// Output daemon-mode's stdout to file
    #[arg(long, value_name = "STDOUT_LOG_FLIE",
        default_value = concat!("/var/log/", env!("CARGO_BIN_NAME"), ".stdout")
    )]
    pub daemon_stdout: Option<std::path::PathBuf>,

    /// Output daemon-mode's stderr to file
    #[arg(long, value_name = "STDERR_LOG_FILE",
        default_value = concat!("/var/log/", env!("CARGO_BIN_NAME"), ".stderr")
    )]
    pub daemon_stderr: Option<std::path::PathBuf>,

    /// merge stdout output to stderr
    #[arg(long, default_value_t = true)]
    pub daemon_merge_stderr: bool,

    /// Path to log file by DEBUG MODE
    #[cfg(debug_assertions)]
    #[arg(
        short = 'L',
        value_name = "ACCESS_LOG_FILE",
        default_value = "./temp/debug.log"
    )]
    pub access_log: std::path::PathBuf,

    /// Path to log file
    #[cfg(not(debug_assertions))]
    #[arg(
        short = 'L',
        value_name = "ACCESS_LOG_FILE",
        default_value = concat!("/var/log/", env!("CARGO_BIN_NAME"), ".log")
    )]
    pub access_log: std::path::PathBuf,

    /// Path to footer file by DEBUG MODE
    #[cfg(debug_assertions)]
    #[arg(long, value_name="FOOTER_FILE.toml", default_value = "share/peercast-root_footer.toml")]
    pub index_txt_footer: Option<std::path::PathBuf>,

    /// Path to footer file
    #[cfg(not(debug_assertions))]
    #[arg(long, value_name="FOOTER_FILE.toml", default_value = None)]
    pub index_txt_footer: Option<std::path::PathBuf>,

    /// Create dummy channel at initialize.(Default true)
    #[arg(long, value_parser, action = clap::ArgAction::Set, default_value_t=true)]
    pub create_dummy_channel: bool,

    /// Create dummy channel at initialize.(Default false)
    #[cfg(not(debug_assertions))]
    #[arg(long, value_parser, action = clap::ArgAction::Set, default_value_t=false)]
    pub create_dummy_channel: bool,

    #[command(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity<clap_verbosity_flag::InfoLevel>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand, Clone)]
pub enum Commands {
    Version {
        #[arg(long, default_value_t = false)]
        json: bool,
    },
}

pub fn version_print(args: &Args) -> anyhow::Result<()> {
    match args.command {
        Some(Commands::Version { json }) => {
            libpeercast_re::util::version_print_with(json, |envs| {
                envs.insert("VERGEN_BIN_NAME", Some(env!("CARGO_BIN_NAME")));
                envs.insert("VERGEN_BIN_VERSION", Some(env!("CARGO_PKG_VERSION")));
            })?;
            exit(0)
        }
        _ => {}
    }

    Ok(())
}
