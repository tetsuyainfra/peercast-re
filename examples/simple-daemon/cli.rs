use clap::{Parser, Subcommand};

use clap_verbosity_flag::{ErrorLevel, Verbosity};

/// Simple Daemon Program
#[derive(Parser, Debug)]
#[command(name = env!("CARGO_BIN_NAME"))]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Enable daemon-mode
    #[arg(short = 'D', long, default_value_t = false)]
    pub daemon: bool,

    /// Output daemon-mode stdout to file
    #[arg(long, value_name = "STDOUT_LOG_FLIE",
        default_value = concat!("/var/log/", env!("CARGO_BIN_NAME"), ".stdout")
    )]
    pub daemon_stdout: Option<std::path::PathBuf>,

    /// Output daemon-mode stderr to file
    #[arg(long, value_name = "STDERR_LOG_FILE",
        default_value = concat!("/var/log/", env!("CARGO_BIN_NAME"), ".stderr")
    )]
    pub daemon_stderr: Option<std::path::PathBuf>,

    /// Path to log file
    #[arg(
        short = 'L',
        value_name = "ACCESS_LOG_FILE",
        default_value = concat!("/var/log/", env!("CARGO_BIN_NAME"), ".log")
    )]
    pub access_log: std::path::PathBuf,

    #[command(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity<clap_verbosity_flag::InfoLevel>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Version {
        #[arg(long, default_value_t = false)]
        json: bool,
    },
}
