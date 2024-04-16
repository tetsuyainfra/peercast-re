#![allow(unused_imports, unused)]
/// peercast-port-checkerd
/// PeerCastのポートが開いているか確認してくれるAPIサーバー
/// IPv4/IPv6の両方のポートを開いて待つ
///
/// API Serverの仕様
/// HTTP Headerに X-Request-Id を持っていればそれを利用し、無ければ自動で生成する
/// *** index.txtの仕様 https://w.atwiki.jp/pecapiracy/pages/14.html
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
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

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
    #[arg(short, long, default_value = "0.0.0.0")]
    bind: std::net::IpAddr,

    #[arg(short, long, default_value_t = 7144)]
    port: u16,

    #[arg(long, default_value = "127.0.0.1")]
    api_bind: std::net::IpAddr,

    #[arg(long, default_value_t = 7143)]
    api_port: u16,

    #[arg(long, default_value_t = 3000)]
    connect_timeout: u64,
}

#[tokio::main]
async fn main() {
    let registry = tracing_subscriber::registry()
        // .with(tracing_subscriber::fmt::layer().with_target(false))
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_line_number(true),
        )
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "peercast_root=info".into())
                .add_directive("hyper=info".parse().unwrap())
                .add_directive("tower_http=info".parse().unwrap())
                .add_directive("axum::rejection=trace".parse().unwrap()),
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

    let args = Args::parse();

    let _state = Arc::new(AppConf {
        connect_timeout: args.connect_timeout,
    });

    let channel_manager: ChannelStore<TrackerChannel> = ChannelStore::new(None, None);
    let arc_channel_manager = Arc::new(channel_manager);

    let listener = tokio::net::TcpListener::bind((args.bind, args.port))
        .await
        .unwrap();
    info!("listening on pcp://{}", listener.local_addr().unwrap(),);

    let api_listener = tokio::net::TcpListener::bind((args.api_bind, args.api_port))
        .await
        .unwrap();
    info!("listening on http://{}", api_listener.local_addr().unwrap(),);

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
