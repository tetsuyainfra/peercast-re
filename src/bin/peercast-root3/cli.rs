use std::process::exit;

use clap::{Parser, Subcommand};

/// Simple Daemon Program
#[derive(Parser, Debug)]
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

pub fn version_print(args: &Args) -> anyhow::Result<()> {
    match args.command {
        Some(Commands::Version { json }) => {
            _version_print(json)?;
            exit(0)
        }
        _ => {}
    }

    Ok(())
}

fn _version_print(output_as_json: bool) -> anyhow::Result<()> {
    use std::collections::BTreeMap;

    if output_as_json {
        let build_envs = vergen_pretty::vergen_pretty_env!()
            .into_iter()
            .filter_map(|(k, v)| v.map(|v| (k, v)))
            .collect::<BTreeMap<_, _>>();
        let s = serde_json::to_string_pretty(&build_envs)?;
        println!("{}", s);
    } else {
        let stdout = std::io::stdout();
        let mut stdout = stdout.lock();

        let mut build_envs = vergen_pretty::vergen_pretty_env!();
        build_envs.insert("VERGEN_BIN_NAME", Some(env!("CARGO_BIN_NAME")));
        build_envs.insert("VERGEN_BIN_VERSION", Some(peercast_re::PKG_VERSION));
        build_envs.insert("VERGEN_PKG_VERSION", Some(peercast_re::PKG_VERSION));
        build_envs.insert(
            "VERGEN_PKG_VERSION_MAJOR",
            Some(peercast_re::PKG_VERSION_MAJOR),
        );
        build_envs.insert(
            "VERGEN_PKG_VERSION_MINOR",
            Some(peercast_re::PKG_VERSION_MINOR),
        );
        build_envs.insert(
            "VERGEN_PKG_VERSION_PATCH",
            Some(peercast_re::PKG_VERSION_PATCH),
        );
        build_envs.insert("VERGEN_PKG_AGENT", Some(peercast_re::PKG_AGENT));
        // pub const PKG_AGENT: Lazy<String> =
        //     Lazy::new(|| format!("PeerCast/0.1218 (REv{PKG_VERSION})"));

        let _pp = vergen_pretty::PrettyBuilder::default()
            .env(build_envs)
            .build()?
            .display(&mut stdout)?;
    }

    Ok(())
}
