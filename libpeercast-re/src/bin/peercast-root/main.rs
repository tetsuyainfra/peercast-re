#![allow(unused)]
use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock, RwLock},
    time::{Duration, Instant},
};

use axum::{response::IntoResponse, routing, Json, Router};
use bytes::BytesMut;
use chrono::{DateTime, TimeZone, Utc};
use clap::Parser;
use futures_util::{future::BoxFuture, FutureExt, SinkExt, StreamExt};
use ipnet::IpAdd;
use itertools::concat;
use peercast_re::{
    config,
    pcp::{
        builder::{QuitBuilder, QuitReason, RootBuilder},
        connection::PcpConnection,
        decode::{PcpBroadcast, PcpChannel, PcpHost},
        procedure::PcpHandshake,
        ChannelInfo, GnuId, Id4, ParentAtom, PcpConnectionFactory, TrackInfo,
    },
    util::{
        identify_protocol, mutex_poisoned, rwlock_read_poisoned, rwlock_write_poisoned,
        ConnectionProtocol,
    },
    ConnectionId,
};
use peercast_re_api::models::channel_info;
use repository::{Channel, ChannelRepository};
use rml_rtmp::handshake::Handshake;
use serde::Serialize;
use tokio::{
    fs::read,
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
    time::Interval,
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument::WithSubscriber, trace, warn};
use url::Url;

// use crate::channel::{tracker_channel::TrackerChannel, ChannelStore};

mod cli;
mod logging;
mod repository;
mod shutdown;

#[cfg(test)]
mod test_helper;

// Don't use directly. SEE: REPOSITORY()
static _REPOSITORY: OnceLock<ChannelRepository<RootChannel>> = OnceLock::new();
// Don't use directly. SEE: CONN_FACTORY()
static _CONN_FACTORY: OnceLock<PcpConnectionFactory> = OnceLock::new();
// Don't use directly. SEE: HTTP_API()
static _HTTP_API: OnceLock<Router> = OnceLock::new();
// Don't use directly. SEE: INDEX_TXT_FOOTER()
static _INDEX_TXT_FOOTER: OnceLock<Vec<JsonChannel>> = OnceLock::new();

#[derive(Debug, Clone)]
struct ApiState {}

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
#[allow(non_snake_case)]
pub fn HTTP_API() -> &'static Router {
    _HTTP_API.get().unwrap()
}

#[inline]
#[allow(non_snake_case, private_interfaces)]
pub fn INDEX_TXT_FOOTER() -> &'static Vec<JsonChannel> {
    _INDEX_TXT_FOOTER.get().unwrap()
}

fn init_app(args: &cli::Args, self_session_id: GnuId, self_socket: SocketAddr) {
    _REPOSITORY.get_or_init(|| ChannelRepository::new(&self_session_id));
    //
    _CONN_FACTORY.get_or_init(|| PcpConnectionFactory::new(self_session_id, self_socket));
    //
    _HTTP_API.get_or_init(|| {
        axum::Router::new()
            .route("/", axum::routing::get(root))
            .route("/ws", axum::routing::get(root))
            .with_state(ApiState {})
    });
    //
    _INDEX_TXT_FOOTER.get_or_init(|| {
        let v = match args.index_txt_footer {
            Some(ref p) => {
                // open.p
                todo!()
            }
            None => {
                let mut ch = JsonChannel::empty();
                ch.name = "Powered by peercast-re".into();
                ch.contact_url = "https://beta-yp.007144.xyz/".into();
                vec![ch]
            }
        };
        v
    });

    // DEBUG
    let mut chinfo = ChannelInfo::new();
    chinfo.name = "aiueo><><".into();
    chinfo.url = "http://aiueo/<>".into();
    let config = RootConfig {
        tracker_host: Some("127.0.0.1:7144".parse().unwrap()),
    };
    REPOSITORY().create_or_get(GnuId::new(), Some(chinfo), None, Some(config));

    let mut chinfo = ChannelInfo::new();
    chinfo.name = "あいうえお".into();
    chinfo.url = "http://あいうえお".into();
    REPOSITORY().create_or_get(GnuId::new(), Some(chinfo), None, None);
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
    info!(
        "PCP listening on pcp://{}",
        listener_pcp.local_addr().unwrap(),
    );

    let listener_http: TcpListener =
        tokio::net::TcpListener::bind((args.api_bind, args.api_port)).await?;
    info!(
        "HTTP listening on http://{}",
        listener_http.local_addr().unwrap(),
    );

    let (shutdown_task, graceful, force) = shutdown::create_task_anyhow();

    let shutdown_task = tokio::spawn(shutdown_task);
    // let peercast_server_task = tokio::spawn(server_peercast(
    //     args.clone(),
    //     listener_pcp,
    //     graceful.clone(),
    //     force.clone(),
    // ));
    let http_server_task = tokio::spawn(server_http(args, listener_http, graceful, force));

    // futures_util::future::join_all(vec![http_server_task])
    futures_util::future::join_all(vec![shutdown_task, http_server_task])
        // futures_util::future::join_all(vec![peercast_server_task, http_server_task])
        // futures_util::future::join_all(vec![shutdown_task, peercast_server_task, http_server_task])
        .await;

    Ok(())
}

async fn server_peercast(
    args: cli::Args,
    listener: TcpListener,
    graceful_shutdown: CancellationToken,
    force_shutdown: CancellationToken,
) -> anyhow::Result<()> {
    let tracker = tokio_util::task::TaskTracker::new();
    info!("START PCP SERVER");

    let app: axum::Router =
        axum::Router::new().route("/", axum::routing::get(|| async { "/ path" }));

    loop {
        let cid = ConnectionId::new();
        let name = format!("tcp({})", cid);
        let spawner = tokio::task::Builder::new().name(&name);
        let child_graceful_shutdown = graceful_shutdown.child_token();
        let child_force_shutdown = force_shutdown.child_token();

        tokio::select! {
            accept = listener.accept() => {
                match accept {
                    Ok((stream, addr)) => {
                        let _handle = spawner.spawn(tracker.track_future(serve_peercast( cid, stream, addr, child_graceful_shutdown, child_force_shutdown)));
                    }
                    Err(e) => {
                        error!(?e, "something is occured in listener.accept()");
                        break;
                    }
                }
            },
            _ = graceful_shutdown.cancelled() => {
                info!("GRACEFUL SHUTDOWN REQUESTED");
                break;
            }
        }
    }

    tokio::select! {
        _ = force_shutdown.cancelled() => {
            // trackerが存在することでくれるありがたい
            tracker.close();
            tracker.wait().await;

            // MEMO: こうやってshutdownのタイムアウトを設定してもいいよな・・・
            // timeout(tracker.wait()).await
        }
    }

    Ok(())
}

#[inline]
async fn serve_peercast(
    cid: ConnectionId,
    mut stream: TcpStream,
    remote: SocketAddr,
    graceful_shutdown: CancellationToken,
    force_shutdown: CancellationToken,
) {
    info!(?cid, ?remote, "SPAWN SERVE");
    match identify_protocol(&stream).await {
        Ok(ConnectionProtocol::PeerCast) => {
            serve_root(cid, stream, remote, graceful_shutdown, force_shutdown).await
        }
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
    cid: ConnectionId,
    mut stream: TcpStream,
    remote: SocketAddr,
    graceful_shutdown: CancellationToken,
    force_shutdown: CancellationToken,
) {
    use peercast_re::pcp::connection::HandshakeType;
    let read_buf = BytesMut::new();

    // HandshakeFutureにすればよさそう
    let handshake = CONN_FACTORY().accept(cid, stream, remote);

    // Handshake時に送ってもらうAtomを作成する
    let root_atom = RootBuilder::default()
        .set_update_interval(10)
        .set_next_update_interval(10)
        .build();

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
    // TODO: HostのIPチェックを行う

    // Hostの接続先を確定
    let tracker_host = host
        .as_ref()
        .and_then(|pcp_host| get_tracker_addr(&remote, pcp_host));

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
    let config = RootConfig { tracker_host };

    // 対象チャンネルを取得
    let repo = REPOSITORY();
    let ch = repo.create_or_get(*channel_id_in_bcst, channel_info, track_info, Some(config));

    // Channelにコネクションを接続
    let attach_task = ch.attach_connection(conn);
    attach_task.await;
}

#[derive(Debug, Clone)]
pub struct RootChannel {
    cid: GnuId,
    tracker_addr: Arc<RwLock<Option<SocketAddr>>>,
    channel_info: Arc<RwLock<peercast_re::pcp::ChannelInfo>>,
    track_info: Arc<RwLock<peercast_re::pcp::TrackInfo>>,
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
        channel_info: Option<peercast_re::pcp::ChannelInfo>,
        track_info: Option<peercast_re::pcp::TrackInfo>,
        config: Option<RootConfig>,
    ) -> Self {
        let tracker_addr = config.and_then(|c| c.tracker_host);
        let now_ = Utc::now();

        Self {
            cid,
            tracker_addr: Arc::new(RwLock::new(tracker_addr)),
            channel_info: RwLock::new(channel_info.unwrap_or_default()).into(),
            track_info: RwLock::new(track_info.unwrap_or_default()).into(),
            last_update: Arc::new(Mutex::new(now_.clone())),
            created_at: Arc::new(now_),
        }
    }

    fn last_update(&self) -> DateTime<Utc> {
        self.last_update
            .lock()
            .unwrap_or_else(mutex_poisoned)
            .clone()
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
        } = &bcst;
        if channel_packet.is_none() {
            return;
        }
        // Host情報の更新
        {
            let tracker_host = host
                .as_ref()
                .and_then(|pcp_host| get_tracker_addr(remote_addr, pcp_host));
            let mut tracker_host_locked = self
                .tracker_addr
                .write()
                .unwrap_or_else(rwlock_write_poisoned);
            *tracker_host_locked = tracker_host;
        }

        //
        let PcpChannel {
            channel_info,
            track_info,
            ..
        } = channel_packet.as_ref().unwrap();
        {
            let mut channel_info_locked = self
                .channel_info
                .write()
                .unwrap_or_else(rwlock_write_poisoned);
            let mut track_info_locked = self
                .track_info
                .write()
                .unwrap_or_else(rwlock_write_poisoned);
            let mut last_update_locked = self.last_update.lock().unwrap_or_else(mutex_poisoned);

            match channel_info.as_ref() {
                Some(new_channel_info) => {
                    channel_info_locked.merge_pcp(new_channel_info);
                }
                None => (),
            };
            match track_info.as_ref() {
                Some(new_track_info) => {
                    track_info_locked.merge_pcp(new_track_info);
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
        self.channel_info
            .read()
            .unwrap_or_else(rwlock_read_poisoned)
            .clone()
    }
    fn track_info(&self) -> TrackInfo {
        self.track_info
            .read()
            .unwrap_or_else(rwlock_read_poisoned)
            .clone()
    }

    fn tracker_addr(&self) -> Option<SocketAddr> {
        self.tracker_addr
            .read()
            .unwrap_or_else(rwlock_read_poisoned)
            .clone()
    }
    fn created_at(&self) -> DateTime<Utc> {
        self.created_at.as_ref().clone()
    }
}

impl RootChannel {
    fn attach_connection(self, mut pcp_connection: PcpConnection) -> AttachTaskFuture {
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
                };
            } // loop 'main

            let quit = QuitBuilder::new(QuitReason::Any).build();
            write_inner.write_atom(quit).await;
            info!("QUIT Channel");

            drop(read_inner);
            drop(write_inner);
        }
        .boxed()
    }
}

type AttachTaskFuture = BoxFuture<'static, ()>;

fn get_tracker_addr(remote_addr: &SocketAddr, pcp_host: &PcpHost) -> Option<SocketAddr> {
    // Hostの接続先を確定
    // TODO: firewall checkが必要
    let host = pcp_host
        .addresses
        .iter()
        .find(|addr| addr.ip() == remote_addr.ip());
    let tracker_host = host.map(|h| h.clone());

    tracker_host
}

//-------------------------------------------------------------------------------
// HTTP
//-------------------------------------------------------------------------------
async fn server_http(
    args: cli::Args,
    listener: TcpListener,
    graceful_shutdown: CancellationToken,
    force_shutdown: CancellationToken,
) -> anyhow::Result<()> {
    use axum::routing::any;
    use tower_http::{
        services::ServeDir,
        trace::{DefaultMakeSpan, TraceLayer},
    };

    let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    println!("asset_dir: {:?}", &assets_dir);

    let tracker = tokio_util::task::TaskTracker::new();
    info!("START HTTP SERVER");

    let app = Router::new()
        .fallback_service(ServeDir::new(assets_dir).append_index_html_on_directories(true))
        .route("/index.txt", routing::get(index_txt))
        .route("/api/index.json", routing::get(index_json))
        // logging so we can see what's going on
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        );

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal(graceful_shutdown, force_shutdown))
    .await
    .unwrap();

    Ok(())
}

fn shutdown_signal(
    graceful_shutdown: CancellationToken,
    _force_shutdown: CancellationToken,
) -> BoxFuture<'static, ()> {
    async move {
        //
        graceful_shutdown.cancelled().await;
        info!("HTTP start graceful shutdown");
    }
    .boxed()
}

async fn index_txt() -> impl IntoResponse {
    let channels: Vec<JsonChannel> = REPOSITORY().map_collect(|(id, ch)| ch.into());
    let channels = channels.iter();
    let footer = INDEX_TXT_FOOTER().iter();
    let channels = channels.chain(footer).map(|c| c.to_line_of_index_txt());

    itertools::join(channels, "\n")
}

async fn index_json() -> Json<Vec<JsonChannel>> {
    let mut channels: Vec<JsonChannel> = REPOSITORY().map_collect(|(id, ch)| ch.into());

    channels.reserve(INDEX_TXT_FOOTER().len());
    channels.extend(INDEX_TXT_FOOTER().clone());

    channels.into()
}

#[derive(Debug, Clone, Serialize)]
struct JsonChannel {
    id: GnuId,
    name: String,
    tracker_addr: Option<SocketAddr>,
    contact_url: String,
    genre: String,
    desc: String,
    comment: String,
    stream_type: String,
    stream_ext: String,
    bitrate: i32,
    // filetype: String,
    // status: String,
    number_of_listener: i32,
    number_of_relay: i32,
    created_at: DateTime<Utc>,
    track: JsonTrack,
}

#[derive(Debug, Clone, Serialize)]
struct JsonTrack {
    title: String,
    creator: String,
    url: String,
    album: String,
    genre: String,
}

impl From<&RootChannel> for JsonChannel {
    fn from(ch: &RootChannel) -> Self {
        let ChannelInfo {
            typ,
            name,
            genre,
            desc,
            comment,
            url,
            stream_type,
            stream_ext,
            bitrate,
        } = ch.channel_info();

        JsonChannel {
            id: ch.id(),
            name,
            tracker_addr: ch.tracker_addr(),
            contact_url: url,
            genre,
            desc,
            comment,
            stream_type,
            stream_ext,
            bitrate,
            number_of_listener: 10, //
            number_of_relay: 10,    //
            created_at: ch.created_at(),
            track: ch.track_info().into(),
        }
    }
}

impl From<TrackInfo> for JsonTrack {
    fn from(t: TrackInfo) -> Self {
        JsonTrack {
            title: t.title,
            creator: t.creator,
            url: t.url,
            album: t.album,
            genre: t.genre,
        }
    }
}

impl JsonChannel {
    fn to_line_of_index_txt(&self) -> String {
        create_index_line(
            &self.name,
            &self.id,
            &self.tracker_addr,
            &self.contact_url,
            &self.genre,
            &self.desc,
            self.number_of_listener,
            self.number_of_relay,
            self.bitrate,
            &self.stream_ext,
            &self.created_at,
            &self.comment,
        )
    }
    fn empty() -> Self {
        println!("DATETIME              {}", Utc.timestamp_opt(0, 0).unwrap());
        Self {
            id: GnuId::NONE,
            name: "".into(),
            tracker_addr: None,
            contact_url: "".into(),
            genre: "".into(),
            desc: "".into(),
            comment: "".into(),
            stream_type: "".into(),
            stream_ext: "".into(),
            bitrate: 0,
            number_of_listener: 0,
            number_of_relay: 0,
            created_at: Utc.timestamp_opt(0, 0).unwrap(),
            track: JsonTrack {
                title: "".into(),
                creator: "".into(),
                url: "".into(),
                album: "".into(),
                genre: "".into(),
            },
        }
    }
}

fn create_index_line(
    name: &String,
    id: &GnuId,
    tracker_addr: &Option<SocketAddr>,
    contact_url: &String,
    genre: &String,
    desc: &String,
    number_of_listener: i32,
    number_of_relay: i32,
    bitrate: i32,
    stream_ext: &String,
    created_at: &DateTime<Utc>,
    comment: &String,
) -> String {
    use html_escape::{encode_quoted_attribute, encode_safe};
    let diff_time = Utc::now() - created_at;
    let hour = diff_time.num_hours();
    let min = diff_time.num_minutes() % 60;

    let addr = tracker_addr
        .as_ref()
        .map(|a| a.to_string())
        .unwrap_or_default();
    format!(
        "{name}<>{id}<>{addr}<>{contact_url}<>{genre}<>{desc}<>{number_of_listener}<>{number_of_relay}<>{bitrate}<>{file_ext}<>\
        <><><><>{name_escaped}<>{time_hour}:{time_min:02}<>click<>{comment}<>",
        name = encode_safe(&name.clone()),
        id = id,
        addr = addr,
        contact_url = encode_quoted_attribute(&contact_url),
        genre = encode_safe(&genre),
        desc = encode_safe(&desc),
        number_of_listener = number_of_listener,
        number_of_relay = number_of_relay,
        bitrate = bitrate,
        file_ext = encode_safe(&stream_ext),
        name_escaped = encode_safe(&name),
        time_hour = hour,
        time_min = min,
        comment = comment
    )
}

/*
async fn serve_http(
    cid: ConnectionId,
    mut stream: TcpStream,
    remote: SocketAddr,
    graceful_shutdown: CancellationToken,
    force_shutdown: CancellationToken,
) {
    let socket = hyper_util::rt::TokioIo::new(stream);

    let hyper_service = hyper_util::service::TowerToHyperService::new(HTTP_API().clone());

    let builder =
        hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new());
    // builder.http2().enable_connect_protocol(); // ENABLE HTTP2
    let conn = builder.serve_connection_with_upgrades(socket, hyper_service);

    futures_util::pin_mut!(conn);
    loop {
        tokio::select! {
            // HTTP1.1以降の接続の使い回しができるようになっている？
            result = conn.as_mut() => {
                if let Err(_err) = result {
                    trace!("failed to serve connection: {_err:#}");
                }
                break;
            }
        }
    }
}

async fn ws_handler(
    ws: axum::extract::WebSocketUpgrade,
    user_agent: Option<axum_extra::TypedHeader<axum_extra::headers::UserAgent>>,
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<SocketAddr>,
) -> impl axum::response::IntoResponse {
    let user_agent = if let Some(axum_extra::TypedHeader(user_agent)) = user_agent {
        user_agent.to_string()
    } else {
        String::from("Unknown browser")
    };
    // println!("`{user_agent}` at {addr} connected.");
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| handle_socket(socket, addr))
}
/// Actual websocket statemachine (one will be spawned per connection)
async fn handle_socket(mut socket: axum::extract::ws::WebSocket, who: SocketAddr) {
    use axum::extract::ws::{CloseFrame, Message, Utf8Bytes, WebSocketUpgrade};
    use bytes::Bytes;

    /*
    socket
        .send(Message::Ping(Bytes::from_static(b"1234")))
        .await;

    // send a ping (unsupported by some browsers) just to kick things off and get a response
    if socket
        .send(Message::Ping(bytes::Bytes::from_static(&[1, 2, 3])))
        .await
        .is_ok()
    {
        println!("Pinged {who}...");
    } else {
        println!("Could not send ping {who}!");
        // no Error here since the only thing we can do is to close the connection.
        // If we can not send messages, there is no way to salvage the statemachine anyway.
        return;
    } */

    // receive single message from a client (we can either receive or send with socket).
    // this will likely be the Pong for our Ping or a hello message from client.
    // waiting for message from a client will block this task, but will not block other client's
    // connections.
    if let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            if process_message(msg, who).is_break() {
                return;
            }
        } else {
            println!("client {who} abruptly disconnected");
            return;
        }
    }

    // Since each client gets individual statemachine, we can pause handling
    // when necessary to wait for some external event (in this case illustrated by sleeping).
    // Waiting for this client to finish getting its greetings does not prevent other clients from
    // connecting to server and receiving their greetings.
    // for i in 1..5 {
    //     if socket
    //         .send(Message::Text(format!("Hi {i} times!").into()))
    //         .await
    //         .is_err()
    //     {
    //         println!("client {who} abruptly disconnected");
    //         return;
    //     }
    //     tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    // }

    // By splitting socket we can send and receive at the same time. In this example we will send
    // unsolicited messages to client based on some sort of server's internal event (i.e .timer).
    let (mut sender, mut receiver) = socket.split();

    // Spawn a task that will push several messages to the client (does not matter what client does)
    let mut send_task = tokio::spawn(async move {
        let n_msg = 20;

        let mut interval = tokio::time::interval(Duration::from_secs(1));
        let mut i = 0;
        loop {
            tokio::select! {
                r =  sender.send(Message::Text(format!("Server message {i} ...").into())) => {
                    if r.is_err() { break }
                }
            }
            i += 1;
            interval.tick().await;
        }
        // for i in 0..n_msg {
        //     // In case of any websocket error, we exit.
        //     if sender
        //         .send(Message::Text(format!("Server message {i} ...").into()))
        //         .await
        //         .is_err()
        //     {
        //         return i;
        //     }

        //     tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        // }

        // println!("Sending close to {who}...");
        // if let Err(e) = sender
        //     .send(Message::Close(Some(CloseFrame {
        //         code: axum::extract::ws::close_code::NORMAL,
        //         reason: Utf8Bytes::from_static("Goodbye"),
        //     })))
        //     .await
        // {
        //     println!("Could not send Close due to {e}, probably it is ok?");
        // }
        // n_msg
        i
    });

    // This second task will receive messages from client and print them on server console
    let mut recv_task = tokio::spawn(async move {
        let mut cnt = 0;
        while let Some(Ok(msg)) = receiver.next().await {
            cnt += 1;
            // print message and break if instructed to do so
            if process_message(msg, who).is_break() {
                break;
            }
        }
        cnt
    });

    // If any one of the tasks exit, abort the other.
    tokio::select! {
        rv_a = (&mut send_task) => {
            match rv_a {
                Ok(a) => println!("{a} messages sent to {who}"),
                Err(a) => println!("Error sending messages {a:?}")
            }
            recv_task.abort();
        },
        rv_b = (&mut recv_task) => {
            match rv_b {
                Ok(b) => println!("Received {b} messages"),
                Err(b) => println!("Error receiving messages {b:?}")
            }
            send_task.abort();
        }
    }

    // returning from the handler closes the websocket connection
    println!("Websocket context {who} destroyed");
}

/// helper to print contents of messages to stdout. Has special treatment for Close.
fn process_message(
    msg: axum::extract::ws::Message,
    who: SocketAddr,
) -> std::ops::ControlFlow<(), ()> {
    use axum::extract::ws::Message;
    use std::ops::ControlFlow;
    match msg {
        Message::Text(t) => {
            println!(">>> {who} sent str: {t:?}");
        }
        Message::Binary(d) => {
            println!(">>> {} sent {} bytes: {:?}", who, d.len(), d);
        }
        Message::Close(c) => {
            if let Some(cf) = c {
                println!(
                    ">>> {} sent close with code {} and reason `{}`",
                    who, cf.code, cf.reason
                );
            } else {
                println!(">>> {who} somehow sent close message without CloseFrame");
            }
            return ControlFlow::Break(());
        }

        Message::Pong(v) => {
            println!(">>> {who} sent pong with {v:?}");
        }
        // You should never need to manually handle Message::Ping, as axum's websocket library
        // will do so for you automagically by replying with Pong and copying the v according to
        // spec. But if you need the contents of the pings you can see them here.
        Message::Ping(v) => {
            println!(">>> {who} sent ping with {v:?}");
        }
    }
    ControlFlow::Continue(())
}
    */
#[cfg(test)]
mod t {
    use crate::{test_helper::*, RootChannel};

    #[test]
    fn check_types() {
        assert_sized::<RootChannel>();
    }
}
