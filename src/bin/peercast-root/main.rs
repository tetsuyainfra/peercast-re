#![allow(unused_imports, unused)]
/// peercast-port-checkerd
/// PeerCastのポートが開いているか確認してくれるAPIサーバー
/// IPv4/IPv6の両方のポートを開いて待つ
///
/// API Serverの仕様
/// HTTP Headerに X-Request-Id を持っていればそれを利用し、無ければ自動で生成する
/// *** index.txtの仕様 <https://w.atwiki.jp/pecapiracy/pages/14.html>
/// >> (1)<>(2)<>(3)<>(4)<>(5)<>(6)<>(7)<>(8)<>(9)<>(10)<>(11)<>(12)<>(13)<>(14)<>(15)<>(16)<>(17)<>(18)<>(19)
/// 1. 配信チャンネル名
/// 2. チャンネルID
/// 3. 配信者のIPアドレス：PeerCast仕様ポート
/// 4. コンタクトURL
/// 5.ジャンル
/// 6. 詳細
/// 7. 視聴者数
/// 8. リレー数
/// 9. ビットレート(kbps)
/// 10. ファイル形式
/// 11.
/// 12.
/// 13.
/// 14.
/// 15. 配信チャンネル名をURLエンコードしたもの
/// 16. 配信時間
/// 17. ステータス(通常はclick)
/// 18. 配信者からのコメント
/// 19.
mod api_server;
mod channel;
mod connection;
mod error;
mod manager;
mod pcp_server;

use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    process::exit,
    sync::{mpsc::channel, Arc, RwLock},
};

use channel::{ChannelTrait, TrackerDetail};
use chrono::{DateTime, Utc};
use clap::Parser;
use futures_util::{
    future::{join_all, select_all, BoxFuture},
    FutureExt,
};
use http::header::SEC_WEBSOCKET_ACCEPT;
use hyper::client::conn;
use minijinja::filters::first;
use pbkdf2::hmac::digest::typenum::Le;
use peercast_re::{
    error::{AtomParseError, HandshakeError},
    pcp::{
        builder::QuitBuilder, decode::PcpBroadcast, Atom, ChannelInfo, GnuId, Id4, PcpConnectType,
        PcpConnection, PcpConnectionFactory, PcpConnectionReadHalf, PcpConnectionWriteHalf,
    },
    util::util_mpsc::mpsc_send,
    ConnectionId,
};
use rml_rtmp::messages;
use thiserror::Error;
use tokio::sync::{
    mpsc::{self, unbounded_channel, UnboundedReceiver, UnboundedSender},
    watch,
};
use tracing::{debug, error, info, trace, warn};
use tracing_subscriber::{
    fmt::{self, writer::MakeWriterExt},
    prelude::*,
    EnvFilter,
};

use crate::{
    channel::{ChannelStore, TrackerChannel, TrackerChannelConfig},
    connection::TrackerConnection,
};

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct AppConf {
    connect_timeout: u64,
}
// type AppState = Arc<AppConf>;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// PeerCast root server address
    #[arg(short, long, default_value = "0.0.0.0")]
    bind: std::net::IpAddr,

    /// PeerCast root server port
    #[arg(short, long, default_value_t = 7144)]
    port: u16,

    /// HTTP API address
    #[arg(long, default_value = "127.0.0.1")]
    api_bind: std::net::IpAddr,

    /// HTTP API port
    #[arg(long, default_value_t = 7143)]
    api_port: u16,

    /// connection timeout (milli secs)
    #[arg(long, default_value_t = 3000)]
    connect_timeout: u64,

    /// Turn on DEBUG mode
    #[arg(long, default_value_t = false)]
    debug: bool,

    /// Write application log to file
    #[arg(long, value_name = "LOG_FILE")]
    log_file: Option<PathBuf>,
    /// Write application log level
    #[arg(long, value_enum, value_name = "LOG_LEVEL", default_value_t = Level::Info)]
    log_level: Level,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum Level {
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

impl From<Level> for tracing::Level {
    fn from(value: Level) -> Self {
        match value {
            Level::Error => tracing::Level::ERROR,
            Level::Warn => tracing::Level::WARN,
            Level::Info => tracing::Level::INFO,
            Level::Debug => tracing::Level::DEBUG,
            Level::Trace => tracing::Level::TRACE,
        }
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    //
    let log_level = args.log_level.into();

    let file_appender = tracing_appender::rolling::hourly("./tmp", "test.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let registry = tracing_subscriber::registry()
        // stdout
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(args.debug)
                .with_file(args.debug)
                .with_line_number(args.debug)
                .with_filter(
                    EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| "peercast_root=info".into()),
                ),
        )
        // ログファイルの出力
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_line_number(true)
                .with_writer(non_blocking.with_max_level(log_level))
                .with_ansi(false),
        );
    registry.init();

    let exe_name = std::env::current_exe()
        .unwrap()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    info!("START {}", exe_name);
    debug!("logging debug");
    trace!("logging trace");

    let _state = Arc::new(AppConf {
        connect_timeout: args.connect_timeout,
    });

    let channel_manager: ChannelStore<TrackerChannel> = ChannelStore::new(None, None);
    let arc_channel_manager = Arc::new(channel_manager);

    let listener = tokio::net::TcpListener::bind((args.bind, args.port))
        .await
        .unwrap();
    info!("PCP listening on pcp://{}", listener.local_addr().unwrap(),);

    let api_listener = tokio::net::TcpListener::bind((args.api_bind, args.api_port))
        .await
        .unwrap();
    info!(
        "API listening on http://{}",
        api_listener.local_addr().unwrap(),
    );

    let fut_pcp = tokio::spawn(pcp_server::start_pcp_server(
        arc_channel_manager.clone(),
        listener,
    ));
    let fut_api = tokio::spawn(api_server::start_api_server(
        arc_channel_manager,
        api_listener,
    ));
    join_all(vec![fut_pcp, fut_api]).await;
}
