use std::{process::exit, vec};

use axum_client_ip::ClientIpSource;
use clap::{Parser, Subcommand};
use peercast_root::{RestrictPortLevel, YpAppendSystemStatus, YpAppendUserStatus};
use url::Url;

#[cfg(not(debug_assertions))]
const DEFAULT_PORT: u16 = 7144;
#[cfg(debug_assertions)]
const DEFAULT_PORT: u16 = 17144;

#[cfg(not(debug_assertions))]
const DEFAULT_API_BIND: &'static str = "127.0.0.1";
#[cfg(debug_assertions)]
const DEFAULT_API_BIND: &'static str = "0.0.0.0";

#[cfg(not(debug_assertions))]
const DEFAULT_API_PORT: u16 = 7143;
#[cfg(debug_assertions)]
const DEFAULT_API_PORT: u16 = 17143;

#[cfg(not(debug_assertions))]
const DEFAULT_ACCESS_LOG_FILE: Option<&'static str> = Some(concat!("/var/log/", env!("CARGO_BIN_NAME"), ".log"));
#[cfg(debug_assertions)]
const DEFAULT_ACCESS_LOG_FILE: Option<&'static str> = Some("./temp/debug.log");

#[cfg(not(debug_assertions))]
const DEFAULT_CREATE_DUMMY_CHANNEL: bool = false;
#[cfg(debug_assertions)]
const DEFAULT_CREATE_DUMMY_CHANNEL: bool = true;

#[cfg(not(debug_assertions))]
const DEFAULT_INDEX_TXT_FOOTER: Option<&'static str> = None;
#[cfg(debug_assertions)]
const DEFAULT_INDEX_TXT_FOOTER: Option<&'static str> = Some("share/peercast-root_footer.toml");

// #[cfg(not(debug_assertions))]
// const DEFAULT_ALLOW_CORS: &'static str = "";
// #[cfg(debug_assertions)]
const DEFAULT_ALLOW_CORS: &'static str = "http://localhost:3000";

#[cfg(not(debug_assertions))]
const DEFAULT_CACHE_MAX_AGE: u32 = 30;
#[cfg(debug_assertions)]
const DEFAULT_CACHE_MAX_AGE: u32 = 0;

/// Simple Daemon Program
#[derive(Parser, Debug, Clone)]
#[command(name = env!("CARGO_BIN_NAME"))]
#[command(version, about, long_about = None)]
pub struct Args {
    /// PeerCast root server address
    #[arg(short, long, default_value = "0.0.0.0")]
    pub bind: std::net::IpAddr,

    /// PeerCast root server port
    #[arg(short, long, default_value_t = DEFAULT_PORT)]
    pub port: u16,

    /// HTTP API address
    #[arg(long, default_value = DEFAULT_API_BIND)]
    pub api_bind: std::net::IpAddr,

    /// HTTP API port
    #[arg(long, default_value_t = DEFAULT_API_PORT)]
    pub api_port: u16,

    /// Trackerのジャンル名で指定するYPの名前
    /// genre: [yp]@Game
    #[arg(long, default_value = "yp")]
    pub yp_name: String,

    /// Trackerのジャンル名に指定できる名前空間を使用できるか
    /// genre: [yp]@Game
    #[arg(long, default_value_t = true)]
    pub yp_name_spaceable: bool,

    /// Listener数を表示にできるか
    /// genre: yp[?]@Game
    #[arg(long, default_value_t = true)]
    pub yp_listerer_hideable: bool,

    /// Portcheckのレベル制限
    /// genre: ypGame ->  制限無し(level=None)
    /// genre: yp[@]Game -> ポート解放をチェックする(level=1)
    /// genre: yp[@@]Game -> 配信ビットレートで表示制限(level=2)
    /// genre: yp[@@@]Game -> 2MBpsで表示制限(yp-limit-speedで設定可能)(level=3)
    #[arg(long, default_value = "port-check")]
    pub yp_restrict_port_level: RestrictPortLevel,

    /// Portcheckの規制速度(KBps単位、500以上)
    #[arg(long, default_value_t = 2000,
         value_parser = clap::value_parser!(u32).range(peercast_root::YP_LIMIT_SPEED_MIN as i64..))]
    pub yp_limit_speed: u32,

    /// チャンネルリストに追加するホスト情報の種類
    #[arg(long, default_value = "default")]
    pub yp_append_user_status: YpAppendUserStatus,

    /// チャンネルリストに追加するシステム情報の種類
    #[arg(long, default_value = "default")]
    pub yp_append_system_status: YpAppendSystemStatus,

    // pub yp_append_user_status: YpAppendUserStatus,
    #[arg(short, long, env, value_parser = clap::builder::ValueParser::new(Url::parse), default_value="sqlite::memory:")]
    pub database_url: Url,

    // TODO: TIMEZONEの実装
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
    #[arg(short = 'L', value_name = "ACCESS_LOG_FILE", default_value = DEFAULT_ACCESS_LOG_FILE)]
    pub access_log: std::path::PathBuf,

    /// Path to footer file by DEBUG MODE
    #[arg(long, value_name = "FOOTER_FILE.toml", default_value = DEFAULT_INDEX_TXT_FOOTER)]
    pub index_txt_footer: Option<std::path::PathBuf>,

    /// Create dummy channel at initialize.
    #[arg(long, value_parser, action = clap::ArgAction::Set, default_value_t=DEFAULT_CREATE_DUMMY_CHANNEL)]
    pub create_dummy_channel: bool,

    /// Append Access-Controll-Allow-Origin 's Values (example: http://example.com,http://example.com:7143)
    #[arg(long, long_help=LONG_HELP_CORS, value_delimiter=',', default_value=DEFAULT_ALLOW_CORS)]
    pub allow_cors: Vec<String>,

    #[arg(long, default_value_t = DEFAULT_CACHE_MAX_AGE)]
    pub cache_max_age: u32,

    // #[arg(long, default_value = "ConnectInfo")]
    // pub ip_source: axum_client_ip::ClientIpSource,
    #[arg(long, value_enum, default_value_t = ClientIpSourceArg::ConnectInfo)]
    pub client_ip_source: ClientIpSourceArg,

    #[command(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity<clap_verbosity_flag::InfoLevel>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum ClientIpSourceArg {
    /// use ConnectInfo(raw socket address)
    ConnectInfo,
    // /// use Forwarded header (RFC7239)
    // RightmostForwarded,
    /// use X-Forwarded-For header(Nginx, Apache, HAProxy, CDNs, LBs, )
    RightmostXForwardedFor,
    /// use X-Real-Ip header(Nginx)
    XRealIp,
    /// use CF-Connecting-IP (Cloudflare)
    CfConnectingIp,
    /// use True-Client-IP (Cloudflare, Akamai)
    TrueClientIp,
}

impl From<ClientIpSourceArg> for ClientIpSource {
    fn from(v: ClientIpSourceArg) -> Self {
        match v {
            ClientIpSourceArg::ConnectInfo => ClientIpSource::ConnectInfo,
            // ClientIpSourceArg::RightmostForwarded => ClientIpSource::RightmostForwarded,
            ClientIpSourceArg::RightmostXForwardedFor => ClientIpSource::RightmostXForwardedFor,
            ClientIpSourceArg::XRealIp => ClientIpSource::XRealIp,
            ClientIpSourceArg::CfConnectingIp => ClientIpSource::CfConnectingIp,
            ClientIpSourceArg::TrueClientIp => ClientIpSource::TrueClientIp,
        }
    }
}

#[derive(Debug, Subcommand, Clone)]
pub enum Commands {
    Version {
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    DB {
        #[command(subcommand)]
        command: DbCommands,
    },
}

#[derive(Debug, Subcommand, Clone)]
pub enum DbCommands {
    /// DBのマイグレーションを実行する
    Prepare,
}

pub fn version_print(args: &Args) -> anyhow::Result<()> {
    match args.command {
        Some(Commands::Version {
            json,
        }) => {
            libpeercast_re::util::version_print_with(json, |envs| {
                envs.insert("VERGEN_BIN_NAME", Some(env!("CARGO_BIN_NAME")));
                envs.insert("VERGEN_BIN_VERSION", Some(env!("CARGO_PKG_VERSION")));
            })?;
            exit(0)
        }
        Some(Commands::DB {
            ref command,
        }) => match command {
            DbCommands::Prepare => {
                // let database_url = args.database_url.to_string();
                // let pool = SqlitePool::connect(&database_url).await?;
                // let repo = SqliteCheckedHostRepository::new(pool);
                // repo.migrate().await?;
                // println!("Database migration completed successfully.");
            }
        },
        _ => {}
    }

    Ok(())
}

const LONG_HELP_CORS: &str = r#"Append Access-Controll-Allow-Origin 's Values (example: http://example.com,http://example.com:7143)
※ URL末尾のスラッシュも関係してくるので注意すること
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_args_yp_limit_speed() {
        let args = Args::try_parse_from(vec!["peercast-root"]).unwrap();
        assert_eq!(args.yp_limit_speed, 2000);
    }

    #[test]
    fn test_args_yp_port_check_level() {
        let args = Args::try_parse_from(vec!["peercast-root"]).unwrap();
        assert_eq!(args.yp_restrict_port_level, RestrictPortLevel::PortCheck);
    }
}
