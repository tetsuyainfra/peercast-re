#![allow(unused)]
use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    process::exit,
    sync::{Arc, Mutex, OnceLock, RwLock},
    time::{Duration, Instant},
};

use anyhow::Context;
use axum::{
    Json, Router,
    extract::Query,
    http::{HeaderValue, Method},
    response::IntoResponse,
    routing,
    serve::Listener,
};
use axum_extra::headers::Header;
use bytes::BytesMut;
use chrono::{DateTime, TimeZone, Utc};
use clap::Parser;
use futures_util::{FutureExt, SinkExt, StreamExt, future::BoxFuture};
use itertools::concat;
use libpeercast_re::{
    ConnectionNo, config,
    pcp::{
        ChannelInfo, GnuId, Id4, ParentAtom, PcpConnectionFactory, TrackInfo,
        builder::{QuitBuilder, QuitReason, RootBuilder},
        connection::PcpConnection,
        decode::{PcpBroadcast, PcpChannel, PcpHost},
        procedure::PcpHandshake,
    },
    util::{ConnectionProtocol, identify_protocol, mutex_poisoned, rwlock_read_poisoned, rwlock_write_poisoned},
};
use peercast_root::{ExitCode, FooterToml, IndexInfo};
// use peercast_re_api::models::channel_info;
use repository::{Channel, ChannelRepository};
use serde::{Deserialize, Serialize};
use serde_json::value::Index;
use serde_with::{NoneAsEmptyString, serde_as};
use tokio::{
    fs::read,
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
    sync::watch,
    time::Interval,
};
use tokio_util::sync::CancellationToken;
use tower_http::{cors::CorsLayer, set_header::SetResponseHeaderLayer};
use tracing::{debug, error, info, instrument::WithSubscriber, trace, warn};
use url::Url;

// use crate::channel::{tracker_channel::TrackerChannel, ChannelStore};

mod api;
mod cli;
mod db;
mod filter;
mod logging;
mod portcheck;
mod repository;
mod shutdown;
mod shutdown2;
mod shutdown3;

#[cfg(test)]
mod test_helper;

// Don't use directly. SEE: REPOSITORY()
static _REPOSITORY: OnceLock<ChannelRepository<RootChannel>> = OnceLock::new();
// Don't use directly. SEE: CONN_FACTORY()
static _CONN_FACTORY: OnceLock<PcpConnectionFactory> = OnceLock::new();
// Don't use directly. SEE: HTTP_API()
static _HTTP_API: OnceLock<Router> = OnceLock::new();
// Don't use directly. SEE: INDEX_TXT_FOOTER()
static _INDEX_TXT_FOOTER: OnceLock<Vec<IndexInfo>> = OnceLock::new();
// Don't use directly. SEE: REDIS_MASTER_KEY()
static _REDIS_MASTER_KEY: OnceLock<String> = OnceLock::new();

#[derive(Debug, Clone)]
struct ApiState {}

#[inline]
#[allow(non_snake_case, private_interfaces)]
pub fn REDIS_MASTER_KEY() -> &'static str {
    _REDIS_MASTER_KEY.get().unwrap()
}

#[inline]
#[allow(non_snake_case)]
pub fn REPOSITORY() -> &'static ChannelRepository<RootChannel> {
    _REPOSITORY.get().unwrap()
}

#[inline]
#[allow(non_snake_case)]
pub fn CONN_FACTORY() -> &'static PcpConnectionFactory {
    _CONN_FACTORY.get().unwrap()
}

#[inline]
#[allow(non_snake_case, private_interfaces)]
pub fn INDEX_TXT_FOOTER() -> &'static Vec<IndexInfo> {
    _INDEX_TXT_FOOTER.get().unwrap()
}

fn init_app(args: &cli::Args, self_session_id: GnuId, self_socket: SocketAddr) {
    _REDIS_MASTER_KEY.get_or_init(|| args.redis_master_key.clone());
    _REPOSITORY.get_or_init(|| ChannelRepository::new(&self_session_id));
    _CONN_FACTORY.get_or_init(|| PcpConnectionFactory::new(self_session_id, self_socket));
    _INDEX_TXT_FOOTER.get_or_init(|| {
        let mut v = vec![];
        if let Some(ref path) = args.index_txt_footer {
            let mut t = FooterToml::from_path(path)
                .with_context(|| {
                    let p = path.display();
                    format!("index.txtのフッターファイル({p})の読み込みに失敗しました。")
                })
                .unwrap();
            let mut infos: Vec<IndexInfo> = t.infomations.into_iter().map(|i| i.into()).collect();
            dbg!(&infos);
            v.append(&mut infos);
        }
        v
    });

    if args.create_dummy_channel {
        let mut chinfo = ChannelInfo::new();
        let level_fmt = match args.yp_restrict_port_level {
            peercast_root::RestrictPortLevel::None => "",
            peercast_root::RestrictPortLevel::PortCheck => "@",
            peercast_root::RestrictPortLevel::BroadcastSpeed => "@@",
            peercast_root::RestrictPortLevel::RestrictSpeed => "@@@",
            v => {
                error!("Invalid port check level: {:?}", v);
                exit(ExitCode::Failure as i32);
            }
        };
        chinfo.name = "ダミーチャンネル".into();
        chinfo.genre = format!("{}{}ダミージャンル", args.yp_name_space, level_fmt).into();
        chinfo.comment = "ダミーチャンネルはおおよそ5分後に消えます".into();
        chinfo.url = "https://yp-dev.007144.xyz/".into();
        chinfo.typ = "RAW".into();
        let config = RootConfig {
            tracker_host: Some("127.0.0.1:7144".parse().unwrap()),
        };
        REPOSITORY().create_or_get(GnuId::new(), Some(chinfo), None, Some(config));
    }
}

async fn root() -> &'static str {
    "Hello, World!"
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = cli::Args::parse();

    cli::version_print(&args)?;
    logging::init(&args)?;

    init_app(&args, GnuId::new(), (args.bind, args.port).into());

    // Init socket
    let listener_pcp = tokio::net::TcpListener::bind((args.bind, args.port)).await?;
    info!("PCP listening on pcp://{}", listener_pcp.local_addr().unwrap(),);

    let listener_http: TcpListener = tokio::net::TcpListener::bind((args.api_bind, args.api_port)).await?;
    info!("HTTP listening on http://{}", listener_http.local_addr().unwrap(),);

    // let (shutdown_task, graceful, force) = shutdown::create_task_anyhow();
    let (shutdown_task, graceful) = shutdown3::create_task_anyhow();

    let shutdown_task = tokio::spawn(shutdown_task);
    let peercast_server_task = tokio::spawn(server_peercast(args.clone(), listener_pcp, graceful.clone()));
    let http_server_task = tokio::spawn(api::server_http(args, listener_http, graceful));

    // futures_util::future::join_all(vec![http_server_task])
    // futures_util::future::join_all(vec![shutdown_task, http_server_task])
    // futures_util::future::join_all(vec![peercast_server_task, http_server_task])
    futures_util::future::join_all(vec![shutdown_task, peercast_server_task, http_server_task]).await;

    Ok(())
}

async fn server_peercast(
    args: cli::Args,
    listener: TcpListener,
    graceful_shutdown: CancellationToken,
) -> anyhow::Result<()> {
    // スレッドの終了を検知するためのチャンネル
    let (closed_tx, closed_rx) = tokio::sync::watch::channel(());
    let tracker = tokio_util::task::TaskTracker::new();
    info!("START PCP SERVER");

    'accept: loop {
        println!("loop start");
        let cid = ConnectionNo::new();
        let name = format!("tcp({})", cid);
        let spawner = tokio::task::Builder::new().name(&name);
        let child_graceful_shutdown = graceful_shutdown.child_token();

        // Dropすることで、全ての接続終了を確認する
        let closed_rx = closed_rx.clone();

        tokio::select! {
            accept = listener.accept() => {
                println!("accepting connection");
                match accept {
                    Ok((stream, addr)) => {
                        println!("{}: accept connection from {}", &name, &addr);
                        let _handle = spawner.spawn(tracker.track_future(serve_peercast( cid, stream, addr, child_graceful_shutdown, closed_rx.clone())));
                        // let _handle = spawner.spawn(serve_peercast( cid, stream, addr, child_graceful_shutdown, closed_rx));
                    }
                    Err(e) => {
                        error!(?e, "something is occured in listener.accept()");
                        break 'accept;
                    }
                }
            },
            _ = graceful_shutdown.cancelled() => {
                info!("GRACEFUL SHUTDOWN REQUESTED");
                break 'accept;
            }
        };
    }

    // 自身で持っている接続を閉じる
    drop(closed_rx);
    // 全ての接続が閉じるまで待つ
    let _ = closed_tx.closed().await;

    // trackerを閉じて、新規にSpawnできないようにし、全てのスレッドが終了するのを待つ
    tracker.close();
    tracker.wait().await;

    Ok(())
}

#[inline]
async fn serve_peercast(
    cid: ConnectionNo,
    mut stream: TcpStream,
    remote: SocketAddr,
    graceful_shutdown: CancellationToken,
    closed_send: watch::Receiver<()>,
) {
    info!(?cid, ?remote, "SPAWN SERVE");
    match identify_protocol(&stream).await {
        Ok(ConnectionProtocol::PeerCast) => serve_root(cid, stream, remote, graceful_shutdown, closed_send).await,
        Ok(ConnectionProtocol::PeerCastHttp) => {
            error!("PeerCastHttp is not allowed");
            let _ = stream.shutdown().await;
        }
        Ok(ConnectionProtocol::Http) => {
            warn!(?cid, ?remote, "STREAM is HTTP Protocol");
            // serve_http(cid, stream, remote, graceful_shutdown, force_shutdown).await
            let _ = stream.shutdown().await;
        }
        Ok(ConnectionProtocol::Unknown) => {
            warn!(?cid, ?remote, "STREAM is Unkwon Protocol");
            let _ = stream.shutdown().await;
        }
        Err(e) => {
            error!(?cid, ?remote, "Failed: identify_protocol: {}", e);
            let _ = stream.shutdown().await;
        }
    }
}

//-------------------------------------------------------------------------------
// PCP
//-------------------------------------------------------------------------------

async fn serve_root(
    cid: ConnectionNo,
    mut stream: TcpStream,
    remote: SocketAddr,
    graceful_shutdown: CancellationToken,
    closed_send: watch::Receiver<()>,
) {
    use libpeercast_re::pcp::connection::HandshakeType;
    let read_buf = BytesMut::new();

    // HandshakeFutureにすればよさそう
    let handshake = CONN_FACTORY().accept(cid, stream, remote);

    // Handshake時に送ってもらうAtomを作成する
    let root_atom = RootBuilder::default().set_update_interval(10).set_next_update_interval(10).build();

    let mut conn = match handshake.incoming(root_atom.into()).await {
        Err(e) => {
            todo!();
            return;
        }
        Ok(HandshakeType::Ping) => return,
        Ok(HandshakeType::YellowPage(conn)) => conn,
    };

    // RootならTrackerに次の情報を送って、情報のアップデートを求める(Broadcastを遅らせる)
    let root_atom = RootBuilder::build_update_request();
    conn.write_atom(root_atom).await;

    // 最初のAtomはBroadcastが確定する
    let first_atom = match conn.read_atom().await {
        Ok(a) => a,
        Err(_) => return,
    };
    dbg!(&first_atom);

    let bcst = match PcpBroadcast::parse(&first_atom) {
        Ok(b) => b,
        Err(_) => return,
    };
    dbg!(&bcst);

    // パケットの中身が適正か確認する
    let PcpBroadcast {
        channel_id,
        channel_packet,
        host,
        ..
    } = &bcst;
    let (channel_id_in_bcst, channel_packet) = match (channel_id, channel_packet) {
        (Some(chid), Some(chpkt)) => (chid, chpkt),
        _ => return,
    };
    // TODO: HostのIPチェックを行う？

    // Hostの接続先を確定
    let tracker_host = host.as_ref().and_then(|pcp_host| get_tracker_addr(&remote, &pcp_host.addresses));

    let PcpChannel {
        channel_id,
        broadcast_id,
        channel_info,
        track_info,
        ..
    } = channel_packet;

    let (channel_id_in_chpkt, braodcast_id) = match (channel_id, broadcast_id) {
        (Some(chid), Some(bcid)) => (chid, bcid),
        _ => return,
    };

    // 不正チェック
    if channel_id_in_bcst != channel_id_in_chpkt {
        return;
    }

    // チャンネル情報の変換
    let channel_info = channel_info.as_ref().map(|i| i.into());
    let track_info = track_info.as_ref().map(|t| t.into());
    //
    let config = RootConfig {
        tracker_host,
    };

    // 対象チャンネルを取得
    let repo = REPOSITORY();
    let ch = repo.create_or_get(*channel_id_in_bcst, channel_info, track_info, Some(config));

    // Channelにコネクションを接続
    let attach_task = ch.attach_connection(conn, graceful_shutdown, closed_send);
    attach_task.await;
}

#[derive(Debug, Clone)]
pub struct RootChannel {
    cid: GnuId,
    tracker_addr: Arc<RwLock<Option<SocketAddr>>>,
    channel_info: Arc<RwLock<libpeercast_re::pcp::ChannelInfo>>,
    track_info: Arc<RwLock<libpeercast_re::pcp::TrackInfo>>,
    number_of_listener: Arc<RwLock<i32>>,
    number_of_relay: Arc<RwLock<i32>>,
    last_update: Arc<Mutex<DateTime<Utc>>>,
    created_at: Arc<DateTime<Utc>>,
}
#[derive(Debug)]
pub struct RootConfig {
    tracker_host: Option<SocketAddr>,
}

impl Channel for RootChannel {
    type Config = RootConfig;
    fn new(
        self_session_id: GnuId,
        cid: GnuId,
        channel_info: Option<libpeercast_re::pcp::ChannelInfo>,
        track_info: Option<libpeercast_re::pcp::TrackInfo>,
        config: Option<RootConfig>,
    ) -> Self {
        let tracker_addr = config.and_then(|c| c.tracker_host);
        let now_ = Utc::now();

        Self {
            cid,
            tracker_addr: Arc::new(RwLock::new(tracker_addr)),
            channel_info: RwLock::new(channel_info.unwrap_or_default()).into(),
            track_info: RwLock::new(track_info.unwrap_or_default()).into(),
            number_of_listener: RwLock::new(0).into(),
            number_of_relay: RwLock::new(0).into(),
            last_update: Arc::new(Mutex::new(now_.clone())),
            created_at: Arc::new(now_),
        }
    }

    fn last_update(&self) -> DateTime<Utc> {
        self.last_update.lock().unwrap_or_else(mutex_poisoned).clone()
    }
}
impl RootChannel {
    fn arrived_broadcast(&self, bcst: PcpBroadcast, remote_addr: &SocketAddr) {
        info!(cid = ?self.cid, "ArrivedBroadcast");
        debug!(?bcst);
        // TODO: 不正チェックした方がいいかも

        let PcpBroadcast {
            channel_packet,
            host,
            ..
        } = bcst;
        if channel_packet.is_none() {
            return;
        }

        // Host情報の更新
        if let Some(pcp_host) = host {
            let PcpHost {
                addresses,
                number_listener,
                number_relay,
                ..
            } = pcp_host;
            // TrackerのIPアドレスを更新
            {
                let tracker_addr = get_tracker_addr(&remote_addr, &addresses);
                // MEMO: tracker_addr=Noneが帰ってきたらどする？
                let mut tracker_host_locked = self.tracker_addr.write().unwrap_or_else(rwlock_write_poisoned);
                *tracker_host_locked = tracker_addr;
            }
            // listener数を更新
            {
                let mut listner_locked = self.number_of_listener.write().unwrap_or_else(rwlock_write_poisoned);
                if let Some(listner) = number_listener {
                    *listner_locked = listner;
                }
            }
            // relay数を更新
            {
                let mut relay_locked = self.number_of_relay.write().unwrap_or_else(rwlock_write_poisoned);
                if let Some(relay) = number_relay {
                    *relay_locked = relay;
                }
            }
        }

        //
        let PcpChannel {
            channel_info,
            track_info,
            ..
        } = channel_packet.unwrap();
        {
            let mut channel_info_unlocked = self.channel_info.write().unwrap_or_else(rwlock_write_poisoned);
            let mut track_info_unlocked = self.track_info.write().unwrap_or_else(rwlock_write_poisoned);
            let mut last_update_locked = self.last_update.lock().unwrap_or_else(mutex_poisoned);

            match channel_info {
                Some(new_channel_info) => {
                    debug!(?new_channel_info);
                    channel_info_unlocked.merge_pcp(new_channel_info);
                }
                None => (),
            };
            match track_info {
                Some(new_track_info) => {
                    debug!(?new_track_info);
                    track_info_unlocked.merge_pcp(new_track_info);
                }
                None => (),
            };
            *last_update_locked = Utc::now();
        }
    }

    fn id(&self) -> GnuId {
        self.cid
    }

    fn channel_info(&self) -> ChannelInfo {
        self.channel_info.read().unwrap_or_else(rwlock_read_poisoned).clone()
    }
    fn track_info(&self) -> TrackInfo {
        self.track_info.read().unwrap_or_else(rwlock_read_poisoned).clone()
    }

    fn tracker_addr(&self) -> Option<SocketAddr> {
        self.tracker_addr.read().unwrap_or_else(rwlock_read_poisoned).clone()
    }
    fn number_of_listener(&self) -> i32 {
        self.number_of_listener.read().unwrap_or_else(rwlock_read_poisoned).clone()
    }
    fn number_of_relay(&self) -> i32 {
        self.number_of_relay.read().unwrap_or_else(rwlock_read_poisoned).clone()
    }
    fn created_at(&self) -> DateTime<Utc> {
        self.created_at.as_ref().clone()
    }
}

impl RootChannel {
    fn attach_connection(
        self,
        mut pcp_connection: PcpConnection,
        graceful_shutdown: CancellationToken,
        closed_send: watch::Receiver<()>,
    ) -> AttachTaskFuture {
        info!("ATTACH CONNECTION TO CHANNEL ");
        async move {
            info!("START");
            let remote_addr = pcp_connection.remote_addr();
            let (mut read_inner, mut write_inner) = pcp_connection.split();

            'main: loop {
                tokio::select! {
                    atom = read_inner.read_atom() => {
                        match atom {
                            Ok(a) => {
                                // *self.last_update.lock().unwrap_or_else(mutex_poisoned) = Utc::now();
                                match PcpBroadcast::parse(&a) {
                                    Ok(pcp) => self.arrived_broadcast(pcp, &remote_addr),
                                    Err(e) => error!(?e, "error"),
                                };
                            },
                            Err(e) => {
                                error!("Read Error: {:?}", e);
                                break 'main
                            }
                        }
                    }
                    _ = graceful_shutdown.cancelled() => {
                        warn!("Shutdown signal catched");
                        break 'main
                    }
                };
            } // loop 'main

            let quit = QuitBuilder::new(QuitReason::Any).build();
            write_inner.write_atom(quit).await;
            info!("QUIT Channel");

            drop(read_inner);
            drop(write_inner);

            // スレッドの終了を知らせる
            drop(closed_send);
        }
        .boxed()
    }
}

type AttachTaskFuture = BoxFuture<'static, ()>;

fn get_tracker_addr(remote_addr: &SocketAddr, addresses: &Vec<SocketAddr>) -> Option<SocketAddr> {
    // Hostの接続先を確定
    // TODO: firewall checkが必要
    let host = addresses.iter().find(|addr| addr.ip() == remote_addr.ip());
    let tracker_host = host.map(|h| h.clone());

    tracker_host
}

#[cfg(test)]
mod t {
    use crate::{RootChannel, test_helper::*};

    #[test]
    fn check_types() {
        assert_sized::<RootChannel>();
    }
}
