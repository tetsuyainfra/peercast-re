#![feature(ip)]
#![allow(unused)]
use std::{
    collections::{HashMap, VecDeque},
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
    time::Instant,
};

use bytes::{Buf, BytesMut};
use hyper::client::conn;
use peercast_re::{
    error::HandshakeError,
    pcp::{
        builder::{HelloBuilder, OkBuilder, OlehBuilder, PingBuilder, PongBuilder, QuitBuilder},
        decode::{self, PcpBroadcast, PcpHelo, PcpPing, PcpPong},
        read_atom, Atom, Channel, FactoryConfig, GnuId, Id4,
    },
    util::mutex_poisoned,
    ConnectionId,
};
use tokio::{
    io::{AsyncWriteExt, ReadHalf, WriteHalf},
    net::TcpStream,
};
use tokio_util::io::InspectReader;
use tracing::{error, info, instrument, trace};

use crate::{channel::RootChannelConfig, store::ChannelRepository, REPOSITORY};

#[derive(Debug, Clone)]
struct ConnectionManager {
    connections: Arc<Mutex<HashMap<ConnectionId, (SocketAddr,)>>>,
}
impl ConnectionManager {
    fn new() -> ConnectionManager {
        Self {
            connections: Default::default(),
        }
    }

    fn register_connection(&self, id: &ConnectionId, addr: SocketAddr) {
        let r = self
            .connections
            .lock()
            .unwrap_or_else(mutex_poisoned)
            .insert(id.clone(), (addr,));
        if let Some(r) = r {
            error!(?id, ?self, "Duplicate Data: {:?}", r);
        }
    }
}

#[derive(Debug, Clone)]
pub struct PcpConnectionFactory {
    /// 通常、ホストしているPeerCastアプリのsession id
    self_session_id: GnuId,
    /// 通常、ホストしているPeerCastアプリのBINDアドレス
    self_addr: IpAddr,
    /// 通常、ホストしているPeerCastアプリのBINDポート
    self_port: u16,
    manager: ConnectionManager,
}

impl PcpConnectionFactory {
    pub fn new(self_session_id: GnuId, self_addr: IpAddr, self_port: u16) -> PcpConnectionFactory {
        Self {
            self_session_id,
            self_addr,
            self_port,
            manager: ConnectionManager::new(),
        }
    }

    pub async fn connect(&self, connect_to: SocketAddr) -> anyhow::Result<PcpHandshake> {
        let stream = TcpStream::connect(connect_to).await?;
        let connection_id = ConnectionId::new();

        self.manager
            .register_connection(&connection_id, connect_to.clone());

        let inner = Inner::new(
            connection_id,
            self.self_session_id,
            stream,
            connect_to,
            None,
        );
        Ok(PcpHandshake {
            inner: inner,
            factory: self.clone(),
        })
    }

    pub fn accept(
        &self,
        connection_id: ConnectionId,
        stream: TcpStream,
        remote_addr: SocketAddr,
    ) -> PcpHandshake {
        self.manager
            .register_connection(&connection_id, remote_addr.clone());

        let inner = Inner::new(
            connection_id,
            self.self_session_id,
            stream,
            remote_addr,
            None,
        );
        PcpHandshake::new(inner, self.clone())
    }

    pub fn ipaddr(&self) -> IpAddr {
        self.self_addr
    }
    pub fn port(&self) -> u16 {
        self.self_port
    }
}

//--------------------------------------------------------------------------------
// Host: Inner内で参照されるRemoteHostの情報
//
#[derive(Debug, Clone)]
struct Host {
    ip: IpAddr,
    port: u16,
}

//--------------------------------------------------------------------------------
// Inner
//
#[derive(Debug)]
struct Inner {
    connection_id: ConnectionId,
    self_session_id: GnuId,
    stream: TcpStream,
    remote: SocketAddr,
    read_buf: BytesMut,
    //
    read_counts: VecDeque<(Instant, u64)>,
    write_counts: VecDeque<(Instant, u64)>,
    // Remote Hostの情報
    rhost: Host,
}

impl Inner {
    pub(super) fn new(
        connection_id: ConnectionId,
        self_session_id: GnuId,
        stream: TcpStream,
        remote: SocketAddr,
        read_buf: Option<BytesMut>,
    ) -> Self {
        Self {
            connection_id,
            self_session_id,
            stream: stream,
            remote: remote,
            read_buf: read_buf.unwrap_or_else(|| BytesMut::with_capacity(4096)),
            read_counts: Default::default(),
            write_counts: Default::default(),
            rhost: Host {
                ip: remote.ip(),
                port: 0, // Port0と同意
            },
        }
    }

    #[inline]
    pub(super) fn connection_id(&self) -> ConnectionId {
        self.connection_id
    }

    #[inline]
    pub(super) fn remote(&self) -> SocketAddr {
        self.remote
    }

    #[inline]
    pub(self) async fn peek(&self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        self.stream.peek(buf).await
    }

    #[inline]
    pub(self) async fn read_atom(&mut self) -> Result<Atom, std::io::Error> {
        read_atom(&mut self.stream, &mut self.read_buf).await
    }

    #[inline]
    pub(self) async fn write_atom(&mut self, atom: Atom) -> Result<(), std::io::Error> {
        //
        atom.write_stream(&mut self.stream).await
    }

    pub(self) fn split(self) -> (InnerReadHalf, InnerWriteHalf) {
        let Self {
            connection_id,
            self_session_id,
            stream,
            remote,
            read_buf,
            read_counts,
            write_counts,
            rhost,
        } = self;
        let (read_half, write_half) = tokio::io::split(stream);

        let inner_read = InnerReadHalf {
            connection_id,
            self_session_id,
            read_half,
            remote,
            read_buf,
            read_counts,
        };
        let inner_write = InnerWriteHalf {
            connection_id,
            self_session_id,
            write_half,
            remote,
            write_counts,
        };
        (inner_read, inner_write)
    }
}

//--------------------------------------------------------------------------------
// InnerReadHalf
//
#[derive(Debug)]
struct InnerReadHalf {
    connection_id: ConnectionId,
    self_session_id: GnuId,
    read_half: ReadHalf<TcpStream>,
    remote: SocketAddr,
    read_buf: BytesMut,
    //
    read_counts: VecDeque<(Instant, u64)>,
}
impl InnerReadHalf {
    #[inline]
    pub(self) async fn read_atom(&mut self) -> Result<Atom, std::io::Error> {
        read_atom(&mut self.read_half, &mut self.read_buf).await
    }
}

//--------------------------------------------------------------------------------
// InnerWriteHalf
//
#[derive(Debug)]
struct InnerWriteHalf {
    connection_id: ConnectionId,
    self_session_id: GnuId,
    write_half: WriteHalf<TcpStream>,
    remote: SocketAddr,
    //write_buf: BytesMut,
    //
    write_counts: VecDeque<(Instant, u64)>,
}
impl InnerWriteHalf {
    #[inline]
    pub(self) async fn write_atom(&mut self, atom: Atom) -> Result<(), std::io::Error> {
        //
        atom.write_stream(&mut self.write_half).await
    }
}

//--------------------------------------------------------------------------------
// PcpHandshake
//
pub struct PcpHandshake {
    inner: Inner,
    factory: PcpConnectionFactory,
}
impl PcpHandshake {
    const PCP_MAGIC_HEAD: &'static [u8; 4] = b"pcp\n";
    fn new(inner: Inner, factory: PcpConnectionFactory) -> Self {
        Self { inner, factory }
    }

    pub async fn ping(mut self) {
        let ping_atoms = PingBuilder::new(self.inner.self_session_id).build();
        for a in ping_atoms {
            let _ = self.inner.write_atom(a).await;
        }

        let oleh = self.inner.read_atom().await;

        let quit = QuitBuilder::new(peercast_re::pcp::builder::QuitReason::UserShutdown).build();
        let _ = self.inner.write_atom(quit).await;
        self.inner.stream.shutdown().await;
    }

    /// is_root_mode: Rootサーバーかどうか
    pub async fn incoming_pcp(
        mut self,
        is_root_mode: bool,
    ) -> Result<PcpConnection, HandshakeError> {
        let mut head = [0_u8; 4];
        let read_n = self.inner.peek(&mut head).await?;
        assert!(read_n == 4);
        if Self::PCP_MAGIC_HEAD != &head {
            trace!(cid = ?self.inner.connection_id(), "NOT COMMING PCP");
            return Err(HandshakeError::Failed);
        }
        trace!(cid = ?self.inner.connection_id(), "COMMING PCP");

        let atom = self.inner.read_atom().await?;
        // trace!("ARRIVED_ATOM: {:?}", &atom);
        if !(atom.id() == Id4::PCP_CONNECT && atom.is_child() && atom.len() == 4) {
            return Err(HandshakeError::Failed);
        }

        // ip_modeってなってるけどどうやらVersionらしい
        let pcp_version = atom.as_child().payload().get_u32_le();
        assert_eq!(pcp_version, 1_u32); // IPv4の決め打ち

        // PCP_HELO
        let atom = self.inner.read_atom().await?;
        // trace!("ARRIVED_ATOM: {:?}", &atom);
        if atom.id() != Id4::PCP_HELO {
            return Err(HandshakeError::Failed);
        }

        if atom.len() == 1 {
            // MEMO: atomが一つしかなければ事実上PingPongになるのでこう処理しているが・・・
            // 本来は以降の手順中にpingを行って、その結果を返したら接続終了するだけ・・・
            // PCP:PingPongプロトコル
            // PingはPCP_HELOが親でchildにSESSION_IDしかないハズ。。。
            // PCP_HELO(PING)
            let pcp_ping = PcpPing::parse(&atom)?;
            trace!("ARRIVED_PING: {:#?}", &pcp_ping);
            Ok(PcpConnection::new(
                self.inner,
                ConnectionType::IncomingPing(pcp_ping),
            ))
        } else {
            // PCP:接続
            // PCP_HELO(normal)を主体とする接続のハズ
            let pcp_helo = PcpHelo::parse(&atom)?;
            trace!("ARRIVED_HELO: {:#?}", &pcp_helo);
            pcp_helo.port.map(|port| self.inner.rhost.port = port);

            // プライベートネットワーク内からのアクセス時、リモートホストは自分自身のGlobalIPとしている
            // match self.inner.rhost.ip {
            //     IpAddr::V4(ref ipv4_addr) => {
            //         if ipv4_addr.is_private() && myGlobalIp {
            //             rhost.ip = myGlobalIp;
            //         }
            //     }
            //     IpAddr::V6(ref ipv6_addr) => todo!(),
            // };

            if pcp_helo.ping.is_some() {
                todo!("pingを実装してくれー");
                // self.inner.rhost.port = pcp_helo.ping.unwrap();
                // // let ping_ok = ping_host(self.inner.rhost, remote_session_id).await;
                // if !(self.inner.rhost.is_local() && ping_ok) {
                //     self.inner.rhost.port = 0;
                // }
            }

            // Oleh Send
            let oleh = OlehBuilder::new(
                self.inner.self_session_id,
                self.inner.rhost.ip,
                self.inner.rhost.port,
            )
            .build();
            self.inner.write_atom(oleh).await?;
            if is_root_mode {}

            //
            Ok(PcpConnection::new(
                self.inner,
                ConnectionType::IncomingBroadcast(pcp_helo),
            ))
        }
    }

    // async fn outgoing_ping(&mut self) -> Result<PcpPong, HandshakeError> {
    //     let ping_atom = PingBuilder::new(self.inner.self_session_id).build();
    //     for a in ping_atom {
    //         let _ = self.inner.write_atom(a).await;
    //     }
    //     let atom = self.inner.read_atom().await?;
    //     let pong_info = PcpPong::parse(&atom)?;
    //     let quit_atom = self.inner.read_atom().await?;
    //     Ok(pong_info)
    // }
}

///
#[derive(Debug)]
enum ConnectionType {
    Outgoing,
    IncomingPing(PcpPing),
    IncomingBroadcast(PcpHelo),
}

//--------------------------------------------------------------------------------
// PcpConnection
//
#[derive(Debug)]
pub struct PcpConnection {
    conn_type: ConnectionType,
    inner: Inner,
}

impl PcpConnection {
    // const PCP_MAGIC_HEAD: &'static [u8; 4] = b"pcp\n";
    fn new(inner: Inner, conn_type: ConnectionType) -> Self {
        Self { inner, conn_type }
    }

    pub async fn run(mut self) {
        match self.conn_type {
            ConnectionType::Outgoing => self._run_outgoing().await,
            ConnectionType::IncomingPing(p) => Self::_run_incoming_ping(p).await,
            ConnectionType::IncomingBroadcast(p) => {
                Self::_run_incoming_broadcast(p, self.inner).await
            }
        }
    }

    #[instrument]
    async fn _run_outgoing(mut self) {
        todo!("Outgoingの場合の処理を記述してください")
    }

    #[instrument]
    async fn _run_incoming_ping(pcp_ping: PcpPing) {
        todo!("pongを返す")
    }

    async fn _run_incoming_broadcast(pcp_helo: PcpHelo, mut inner: Inner) {
        let atom = inner.read_atom().await.expect("AtomParseError");
        let pcp_broadcast = PcpBroadcast::parse(&atom).expect("PcpBroadcastParseError");
        info!("pcp_broadcast {:#?}", pcp_broadcast);

        let remote_session_id = pcp_helo.session_id;
        let remote_broadcast_id = pcp_helo.broadcast_id;

        let mut channel_id = None;
        match pcp_broadcast.channel_packet {
            None => {
                todo!("接続を実装")
            }
            Some(pcp_chan) => {
                channel_id = pcp_broadcast.channel_id;
            }
        }

        let (channel_id, remote_broadcast_id) = match (channel_id, remote_broadcast_id) {
            (Some(cid), Some(rbid)) => (cid, rbid),
            (ch_id, br_id) => {
                todo!("認証のためのデータがそろってないので接続終了");
            }
        };

        let channel = REPOSITORY().get_or_create(
            channel_id,
            RootChannelConfig {
                remote_session_id,
                remote_broadcast_id,
            },
        );

        // channel.register_downstream(inner)
    }
}
