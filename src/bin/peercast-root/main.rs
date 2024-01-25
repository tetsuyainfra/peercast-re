#![allow(unused_imports, unused)]
/// peercast-port-checkerd
/// PeerCastのポートが開いているか確認してくれるAPIサーバー
/// IPv4/IPv6の両方のポートを開いて待つ
///
/// API Serverの仕様
/// HTTP Headerに X-Request-Id を持っていればそれを利用し、無ければ自動で生成する
mod api;
mod error;
mod manager;

use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    process::exit,
    sync::{mpsc::channel, Arc, RwLock},
};

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
        .with(tracing_subscriber::fmt::layer().with_target(true))
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "peercast_root=info".into())
                .add_directive("hyper=info".parse().unwrap())
                .add_directive("tower_http=info".parse().unwrap())
                .add_directive("axum::rejection=trace".parse().unwrap()),
        );
    registry.init();

    let exename = std::env::current_exe()
        .unwrap()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    info!("START {}", exename);
    debug!("logging debug");
    trace!("logging trace");

    let args = Args::parse();

    let _state = Arc::new(AppConf {
        connect_timeout: args.connect_timeout,
    });

    let channel_manager: ChannelManager<TrackerChannel> = ChannelManager::new(None, None);
    let arc_channel_manager = Arc::new(channel_manager);

    let listener = tokio::net::TcpListener::bind((args.bind, args.port))
        .await
        .unwrap();
    info!("listening on pcp://{}", listener.local_addr().unwrap(),);

    let api_listener = tokio::net::TcpListener::bind((args.api_bind, args.api_port))
        .await
        .unwrap();
    info!("listening on http://{}", api_listener.local_addr().unwrap(),);

    let fut_pcp = tokio::spawn(start_pcp_server(arc_channel_manager.clone(), listener));
    let fut_api = tokio::spawn(api::start_api_server(arc_channel_manager, api_listener));
    join_all(vec![fut_pcp, fut_api]).await;
}

////////////////////////////////////////////////////////////////////////////////
/// PCP Server
///
async fn start_pcp_server(
    arc_channel_manager: Arc<ChannelManager<TrackerChannel>>,
    listener: tokio::net::TcpListener,
) {
    info!("START PCP SERVER");
    let self_session_id = GnuId::new();
    let factory = PcpConnectionFactory::new(self_session_id);

    loop {
        let channel_manager = arc_channel_manager.clone();
        let (stream, remote) = listener.accept().await.unwrap();
        let pcp_handshake = factory.accept(stream, remote);

        let _x: tokio::task::JoinHandle<Result<(), HandshakeError>> = tokio::spawn(async move {
            let mut pcp_connection = pcp_handshake.incoming_pcp().await?;
            println!("{:#?}", &pcp_connection);

            let tracker_connection: TrackerConnection = match &pcp_connection.con_type {
                PcpConnectType::Outgoing => unreachable!(),
                PcpConnectType::IncomingPing(_ping) => {
                    // pingを返す(まー必要ないはずなんだけどあり得る通信なので)
                    todo!()
                }
                PcpConnectType::IncomingBroadcast(helo) => {
                    // BroadcastIdは配信時にRoot(YP)に送られるID、これを知っているのはRootとTrackerなので認証することができる
                    // とりあえずココではBroadcastIdが存在する事を保障する
                    let broadcast_id = Arc::new(helo.broadcast_id.clone());
                    let Some(broadcast_id) = helo.broadcast_id.map(|g| Arc::new(g)) else {
                        error!(
                            "Helo Atom must have BroadcastId CID: {}",
                            pcp_connection.connection_id()
                        );
                        debug!("first atom: {:#?}", helo);
                        return Err(HandshakeError::Failed);
                    };

                    // ChannelIDはHandshake後、最初のAtom(id=Broadcast)に入っているため、一つ目を必ず読み取らなければならない。
                    // これでどの配信チャンネルに対する配信情報の送信か決定できるようになる
                    let first_atom = pcp_connection.read_atom().await?;
                    let Ok(first_broadcast) = PcpBroadcast::parse(&first_atom).map(|p| Arc::new(p))
                    else {
                        error!(
                            "First PCPPacket must be BroadcastAtom(id=bcst): {}",
                            pcp_connection.connection_id()
                        );
                        debug!("first atom: {:#?}", first_atom);
                        return Err(HandshakeError::Failed);
                    };

                    let Some(channel_id) = first_broadcast.channel_id else {
                        error!("first Broadcast must have ChannelId");
                        debug!("first broadcast: {:#?}", first_broadcast);

                        return Err(HandshakeError::Failed);
                    };

                    let channel = channel_manager.create_or_get(
                        channel_id,
                        TrackerChannelConfig {
                            broadcast_id: broadcast_id.clone(),
                            first_broadcast: first_broadcast.clone(),
                        },
                    );

                    // TrackerConnectionを返す
                    channel.tracker_connection(
                        pcp_connection.connection_id(),
                        broadcast_id,
                        first_broadcast,
                    )
                }
            };

            // TrackerConnectionManagerと接続開始する
            let _ = tracker_connection
                .start_connection_manager(pcp_connection)
                .await;

            Ok(())
        });
    }
}

async fn handle_connection() {}

#[derive(Debug)]
pub struct ChannelManager<C: ChannelTrait> {
    session_id: Arc<GnuId>,
    broadcast_id: Arc<GnuId>,
    channels: Arc<RwLock<HashMap<GnuId, C>>>,
}
impl<C: ChannelTrait> Clone for ChannelManager<C> {
    fn clone(&self) -> Self {
        Self {
            session_id: Arc::clone(&self.session_id),
            broadcast_id: Arc::clone(&self.broadcast_id),
            channels: Arc::clone(&self.channels),
        }
    }
}
pub trait ChannelTrait: Clone {
    type Config;
    fn new(
        self_session_id: Arc<GnuId>,
        self_broadcast_id: Arc<GnuId>,
        channel_id: Arc<GnuId>,
        config: Self::Config,
    ) -> Self;

    // call remove
    fn before_remove(&mut self);
}

impl<C: ChannelTrait> ChannelManager<C> {
    fn new(self_session_id: Option<GnuId>, self_broadcast_id: Option<GnuId>) -> Self {
        let session_id = Arc::new(self_session_id.unwrap_or_else(|| GnuId::new()));
        let broadcast_id = Arc::new(self_broadcast_id.unwrap_or_else(|| GnuId::new()));
        Self {
            session_id,
            broadcast_id,
            channels: Default::default(),
        }
    }

    fn create_or_get(&self, channel_id: GnuId, config: C::Config) -> C {
        self.get(&channel_id)
            .or_else(|| {
                // Channelがなければロックしてから検索して返す
                let mut channels = self.channels.write().unwrap();
                let arc_channel_id = Arc::new(channel_id.clone());
                let ch = channels.entry(channel_id).or_insert_with(|| {
                    C::new(
                        self.session_id.clone(),
                        self.broadcast_id.clone(),
                        arc_channel_id,
                        config,
                    )
                });
                Some(ch.clone())
            })
            .unwrap()
    }

    fn get(&self, channel_id: &GnuId) -> Option<C> {
        self.channels
            .read()
            .unwrap()
            .get(channel_id)
            .map(|c| c.clone())
    }

    fn get_channels(&self) -> Vec<C> {
        self.channels
            .read()
            .unwrap()
            .values()
            .map(|c| c.clone())
            .collect()
    }

    fn remove(&mut self, channel_id: &GnuId) {
        let ch = self.channels.write().unwrap().remove(channel_id);
        ch.map(|mut c| c.before_remove());
    }
}

#[derive(Debug, Clone)]
pub struct TrackerChannel {
    self_session_id: Arc<GnuId>,
    channel_id: Arc<GnuId>,
    broadcast: Arc<PcpBroadcast>,
    config: Arc<TrackerChannelConfig>,
    manager_sender: UnboundedSender<RootManagerMessage>,
    detail_reciever: tokio::sync::watch::Receiver<TrackerDetail>,

    created_at: Arc<DateTime<Utc>>,
    _called_before_remove: bool,
}

impl TrackerChannel {
    fn tracker_connection(
        &self,
        connection_id: ConnectionId,
        remote_broadcast_id: Arc<GnuId>,
        first_broadcast: Arc<PcpBroadcast>,
    ) -> TrackerConnection {
        TrackerConnection::new(
            connection_id,
            self.config.clone(),
            self.manager_sender.clone(),
            remote_broadcast_id,
            first_broadcast,
        )
    }
}

#[derive(Debug, Clone)]
pub struct TrackerChannelConfig {
    broadcast_id: Arc<GnuId>,
    first_broadcast: Arc<PcpBroadcast>,
}

impl ChannelTrait for TrackerChannel {
    type Config = TrackerChannelConfig;

    fn new(
        self_session_id: Arc<GnuId>,
        _self_broadcast_id: Arc<GnuId>,
        channel_id: Arc<GnuId>,
        config: Self::Config,
    ) -> Self {
        let (detail_sender, detail_reciever) = tokio::sync::watch::channel(TrackerDetail {});

        // self_broadcast_idはいらないので無視する
        let manager_sender = RootManager::start(channel_id.clone(), detail_sender);

        TrackerChannel {
            channel_id,
            self_session_id,
            broadcast: config.first_broadcast.clone(),
            config: Arc::new(config),
            manager_sender,
            detail_reciever,

            created_at: Arc::new(Utc::now()),
            _called_before_remove: false,
        }
    }

    fn before_remove(&mut self) {
        if !self._called_before_remove {
            // 色々やる
        }
    }
}

struct RootManager {
    channel_id: Arc<GnuId>,
    broadcast: Option<Arc<PcpBroadcast>>,
    detail_sender: watch::Sender<TrackerDetail>,
    // connection_idとSenderを組み合わせた物
    sender_by_connection_id: HashMap<ConnectionId, mpsc::UnboundedSender<ConnectionMessage>>,
    new_disconnect_futures: Vec<BoxFuture<'static, FutureResult>>,
}

impl std::fmt::Debug for RootManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RootManager")
            .field("channel_id", &self.channel_id)
            .field("broadcast", &self.broadcast)
            .field("sender_by_connection_id", &self.sender_by_connection_id)
            // .field("new_disconnect_futures", &self.new_disconnect_futures)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone)]
struct TrackerDetail {}

enum FutureResult {
    Disconnection {
        connection_id: ConnectionId,
    },
    MessageReceived {
        receiver: UnboundedReceiver<RootManagerMessage>,
        message: Option<RootManagerMessage>,
    },
}

impl RootManager {
    fn start(
        channel_id: Arc<GnuId>,
        detail_sender: watch::Sender<TrackerDetail>,
    ) -> mpsc::UnboundedSender<RootManagerMessage> {
        let (tx, rx) = mpsc::unbounded_channel();

        let manager: RootManager = RootManager {
            channel_id,
            broadcast: None,
            detail_sender,
            //
            sender_by_connection_id: HashMap::new(),
            new_disconnect_futures: Vec::new(),
        };

        let _ = tokio::spawn(manager.main(rx));
        tx
    }

    fn cleanup_connection(&mut self, connection_id: ConnectionId) {
        println!("REMOVE: RootManager is removing {}", connection_id);

        self.sender_by_connection_id.remove(&connection_id);
        //     if let Some(key) = self.key_by_connection_id.remove(&connection_id) {
        //         if let Some(players) = self.players_by_key.get_mut(&key) {
        //             players.remove(&connection_id);
        //         }

        //         if let Some(details) = self.publish_details.get_mut(&key) {
        //             if details.connection_id == connection_id {
        //                 self.publish_details.remove(&key);
        //             }
        //         }
        //     }
    }

    async fn main(mut self, receiver: UnboundedReceiver<RootManagerMessage>) {
        info!("START: RootManager {:?}", &self.channel_id);

        async fn new_receiver_future(
            mut receiver: UnboundedReceiver<RootManagerMessage>,
        ) -> FutureResult {
            let result = receiver.recv().await;
            FutureResult::MessageReceived {
                receiver,
                message: result,
            }
        }

        let mut futures = select_all(vec![new_receiver_future(receiver).boxed()]);

        'manager: loop {
            let (result, _index, remaining_futures) = futures.await;
            let mut new_futures = Vec::from(remaining_futures);

            match result {
                FutureResult::MessageReceived { receiver, message } => {
                    match message {
                        Some(message) => self.handle_message(message),
                        None => {
                            debug!("RootManagerMessage sender is all gone.");
                            break 'manager;
                        }
                    }

                    new_futures.push(new_receiver_future(receiver).boxed());
                }
                FutureResult::Disconnection { connection_id } => {
                    self.cleanup_connection(connection_id)
                }
            };

            for future in self.new_disconnect_futures.drain(..) {
                new_futures.push(future);
            }
            futures = select_all(new_futures);
        }

        info!("FINISH: RootManager {:?}", &self.channel_id);
    }

    fn handle_message(&mut self, message: RootManagerMessage) {
        match message {
            RootManagerMessage::NewConnection {
                connection_id,
                sender,
                disconnection,
            } => self.handle_new_connection(connection_id, sender, disconnection),
            RootManagerMessage::PublishChannel {
                session_id,
                broadcast_id,
                first_broadcast,
            } => self.handle_publish_channel(first_broadcast),
            RootManagerMessage::UpdateChannel { broadcast } => {
                self.handle_update_channel(broadcast)
            }
        }
    }

    // チャンネルに新規チャンネルが接続された
    fn handle_new_connection(
        &mut self,
        connection_id: ConnectionId,
        sender: UnboundedSender<ConnectionMessage>,
        disconnection: UnboundedReceiver<()>,
    ) {
        self.sender_by_connection_id.insert(connection_id, sender);
        self.new_disconnect_futures
            .push(wait_for_client_disconnection(connection_id, disconnection).boxed());
    }

    // チャンネルの配信開始
    fn handle_publish_channel(&mut self, frst_broadcast: Arc<PcpBroadcast>) {
        // if fistbroad cast arrived. we should check broadcast_id is same
    }

    // PcpBroadcastを元にチャンネル情報を更新する
    fn handle_update_channel(&mut self, broadcast: Arc<PcpBroadcast>) {
        let Some(group) = &broadcast.broadcast_group else {
            return;
        };
        match group.has_root() {
            true => (),
            false => return,
        };

        let _ = self.detail_sender.send(TrackerDetail {});
    }
}

async fn wait_for_client_disconnection(
    connection_id: ConnectionId,
    mut receiver: UnboundedReceiver<()>,
) -> FutureResult {
    // The channel should only be closed when the client has disconnected
    while let Some(()) = receiver.recv().await {}

    FutureResult::Disconnection { connection_id }
}

pub enum State {
    Waiting,
    Running,
}

struct TrackerConnection {
    connection_id: ConnectionId,
    config: Arc<TrackerChannelConfig>,
    manager_sender: UnboundedSender<RootManagerMessage>,
    remote_broadcast_id: Arc<GnuId>,
    /// Handshake後、最初のパケット
    first_broadcast: Option<Arc<PcpBroadcast>>,
    state: State,
}

impl TrackerConnection {
    fn new(
        connection_id: ConnectionId,
        config: Arc<TrackerChannelConfig>,
        manager_sender: UnboundedSender<RootManagerMessage>,
        remote_broadcast_id: Arc<GnuId>,
        first_broadcast: Arc<PcpBroadcast>,
    ) -> Self {
        Self {
            connection_id,
            config,
            manager_sender,
            remote_broadcast_id,
            first_broadcast: Some(first_broadcast),
            state: State::Waiting,
        }
    }

    /// ConnectionManagerとの接続を開始する
    async fn start_connection_manager(
        mut self,
        connection: PcpConnection,
    ) -> Result<(), RootError> {
        let remote_broadcast_id = self.remote_broadcast_id.clone();
        let remote_session_id = connection.remote_session_id.clone();
        let connection_id = connection.connection_id();
        let (message_sender, mut message_receiver) = mpsc::unbounded_channel();
        let (_disconnection_sender, disconnection_receiver) = mpsc::unbounded_channel();

        let message = RootManagerMessage::NewConnection {
            connection_id: connection.connection_id(),
            sender: message_sender,
            disconnection: disconnection_receiver,
        };
        if !mpsc_send(&self.manager_sender, message) {
            return Err(RootError::InitFailed);
        }

        let (read_half, write_half) = connection.split();
        let (reader_tx, mut reader_rx) = mpsc::unbounded_channel();
        let (mut writer_tx, writer_rx) = mpsc::unbounded_channel();

        let _ = tokio::spawn(read_routine(reader_tx, read_half));
        let _ = tokio::spawn(write_routine(writer_rx, write_half));

        // Publish Request
        // Handshake後、最初のパケット(本当はstart_connection_managerに引数で与えたいけど複雑になるのでOptionで渡している)
        let first_broadcast = self.first_broadcast.take().unwrap();
        let message = RootManagerMessage::PublishChannel {
            session_id: remote_session_id,
            broadcast_id: remote_broadcast_id,
            first_broadcast,
        };
        if !mpsc_send(&self.manager_sender, message) {
            return Err(RootError::InitFailed);
        }

        // Start
        self.state = State::Running;

        let mut results = vec![];

        let _reason = loop {
            let action = self.handle_session_results(&mut results, &mut writer_tx)?;
            if action == ConnectionAction::Disconnect {
                break;
            }

            tokio::select! {
                atom = reader_rx.recv() => {
                    info!("atom: {:#?}",&atom);
                    match atom {
                        None => break,
                        Some(a) => { results = self.handle_arrived_atom(a)?; }
                    };
                },
                // Managerからメッセージが来たら処理
                manager_message = message_receiver.recv() => {
                    info!("action: {:#?}",&action);
                    match manager_message {
                        None => break,
                        Some(message) => {
                            let (new_results, action) = self.handle_connection_message(message)?;
                            if action == ConnectionAction::Disconnect {
                                break;
                            }
                            results = new_results;
                        }
                    }
                }
            };
        };

        Ok(())
    }

    fn handle_session_results(
        &mut self,
        results: &mut Vec<SessionResult>,
        writer_sender: &mut UnboundedSender<Atom>,
    ) -> Result<ConnectionAction, RootError> {
        if results.len() == 0 {
            return Ok(ConnectionAction::None);
        }

        let mut new_results = Vec::new();
        for result in results.drain(..) {
            match result {
                SessionResult::RaisedEvent(event) => {}
            }
        }

        self.handle_session_results(&mut new_results, writer_sender)?;

        Ok(ConnectionAction::None)
    }

    /// 通信相手から到着したAtomを処理する
    fn handle_arrived_atom(&self, atom: Atom) -> Result<Vec<SessionResult>, RootError> {
        match atom.id() {
            Id4::PCP_BCST => {
                info!("{} ARRIVED_BCST {:?}", self.connection_id, atom);
                Ok(vec![])
            }
            Id4::PCP_QUIT => {
                info!("{} ARRIVED_QUIT {:?}", self.connection_id, atom);
                Ok(vec![])
            }
            _ => {
                warn!("{} UNKNOWN ATOM: {:#?}", self.connection_id, atom);
                Ok(vec![])
            }
        }
    }

    /// StreamManagerから到着したメッセージを処理する
    fn handle_connection_message(
        &mut self,
        msg: ConnectionMessage,
    ) -> Result<(Vec<SessionResult>, ConnectionAction), RootError> {
        todo!()
    }
}

async fn read_routine(
    mut tx: UnboundedSender<Atom>,
    mut read_half: PcpConnectionReadHalf,
) -> Result<(), std::io::Error> {
    let conn_id = read_half.connection_id();
    info!("{conn_id} START READ HALF");
    loop {
        let Ok(atom) = read_half.read_atom().await else {
            break;
        };
        debug!("{conn_id} ARRIVED_ATOM {:?}", atom);
        mpsc_send(&mut tx, atom);
    }
    info!("{conn_id} STOP READ HALF");
    Ok(())
}
async fn write_routine(
    mut rx: UnboundedReceiver<Atom>,
    mut write_half: PcpConnectionWriteHalf,
) -> Result<(), std::io::Error> {
    let conn_id = write_half.connection_id();
    info!("{conn_id} START WRITE HALF");
    loop {
        let atom = rx.recv().await;
        match atom {
            None => break,
            Some(atom) => {
                debug!("{conn_id} WRITE_ATOM {}", atom);
                let _ = write_half.write_atom(atom).await;
            }
        };
    }
    info!("{conn_id} STOP WRITE HALF");
    Ok(())
}

#[derive(Debug, Error)]
enum RootError {
    #[error("initialize failed")]
    InitFailed,

    #[error("atom parse error")]
    AtomParseError(#[from] AtomParseError),
}

/// Connection -> Manager メッセージ
#[derive(Debug)]
pub enum RootManagerMessage {
    NewConnection {
        connection_id: ConnectionId,
        sender: UnboundedSender<ConnectionMessage>,
        disconnection: UnboundedReceiver<()>,
    },

    PublishChannel {
        session_id: Arc<GnuId>,
        broadcast_id: Arc<GnuId>,
        first_broadcast: Arc<PcpBroadcast>,
    },
    UpdateChannel {
        broadcast: Arc<PcpBroadcast>,
    },
}
/// Manager -> Connection メッセージ
#[derive(Debug)]
pub enum ConnectionMessage {
    Ok {},
}

/// Manager内操作
#[derive(Debug)]
enum SessionResult {
    RaisedEvent(ServerSessionEvent),
}

#[derive(Debug)]
enum ServerSessionEvent {
    PublishChannelRequested {
        atom: Arc<Atom>,
        broadcast: Arc<PcpBroadcast>,
    },
    PublishChannelFinished {},
    UpdateChannel {
        atom: Arc<Atom>,
    },
}

/// Session操作等を実行した後の動作
#[derive(Debug, PartialEq, Eq)]
pub enum ConnectionAction {
    None,
    Disconnect,
}
