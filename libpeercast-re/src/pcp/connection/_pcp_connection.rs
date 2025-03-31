use std::{
    collections::VecDeque,
    io::BufRead,
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use atom::Atom;
use bytes::{Buf, BytesMut};
use futures_util::future::IntoFuture;
use hyper::rt::Read;
use tokio::{
    io::{AsyncWriteExt, ReadHalf, WriteHalf},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    time::Instant,
};
use tracing::{error, info, trace};

use crate::{
    error::HandshakeError,
    pcp::{
        atom,
        builder::{HelloBuilder, OkBuilder, OlehBuilder, PingBuilder, PongBuilder, RootBuilder},
        decode::{PcpHelo, PcpPing, PcpPong},
        GnuId, Id4,
    },
    ConnectionId,
};

use super::factory::PcpConnectionFactory;

//--------------------------------------------------------------------------------
//
//

#[derive(Debug)]
pub(super) struct Inner {
    connection_id: ConnectionId,
    self_session_id: GnuId,
    stream: TcpStream,
    remote: SocketAddr,
    read_buf: BytesMut,
    //
    read_counts: VecDeque<(Instant, u64)>,
    write_counts: VecDeque<(Instant, u64)>,
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
            remote,
            read_buf: read_buf.unwrap_or_else(|| BytesMut::with_capacity(4096)),
            read_counts: Default::default(),
            write_counts: Default::default(),
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
        atom::read_atom(&mut self.stream, &mut self.read_buf).await
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
        atom::read_atom(&mut self.read_half, &mut self.read_buf).await
    }
}

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
#[derive(Debug)]
pub struct PcpHandshake {
    inner: Inner,
    factory: PcpConnectionFactory,
}

impl PcpHandshake {
    const PCP_MAGIC_HEAD: &'static [u8; 4] = b"pcp\n";
    pub(super) fn new(inner: Inner, factory: PcpConnectionFactory) -> Self {
        Self { inner, factory }
    }

    pub async fn incoming_http(mut self) -> Result<PcpConnection, HandshakeError> {
        // check http

        // have channel ?

        // start pcp negotiation
        // recieve HeloAtom
        let helo_atom = self._read_atom().await?;
        let helo = PcpHelo::parse(&helo_atom)?;

        // check is port opened?
        let open_port_no: Option<u16> = check_port(
            &self.factory,
            self.inner.remote().ip(),
            helo.port,
            helo.session_id,
        )
        .await;

        // return oleh

        // return ok

        Ok(todo!())
    }
    pub async fn incoming_pcp(mut self) -> Result<PcpConnection, HandshakeError> {
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

        let ip_mode = atom.as_child().payload().get_u32_le();
        assert_eq!(ip_mode, 1_u32); // IPv4の決め打ち

        // PCP_HELO
        let atom = self.inner.read_atom().await?;
        // trace!("ARRIVED_ATOM: {:?}", &atom);
        if atom.len() == 1 {
            // PingはPCP_HELOが親でchildにSESSION_IDしかないハズ。。。
            // PCP_HELO(PING)
            let ping_info = PcpPing::parse(&atom)?;
            Ok(PcpConnection::new(
                self.inner,
                ping_info.session_id,
                PcpConnectType::IncomingPing(ping_info),
                None,
            ))
        } else {
            // PCP_HELO(normal)を主体とする接続のハズ
            let helo_info = PcpHelo::parse(&atom)?;
            // trace!("ARRIVED_HELO: {:#?}", &helo_info);
            // Send Oleh, Root, Ok
            let remote_port = self._incoming_pcp_root(&helo_info).await?;
            Ok(PcpConnection::new(
                self.inner,
                helo_info.session_id,
                PcpConnectType::IncomingBroadcast(helo_info),
                remote_port,
            ))
        }
    }

    async fn _incoming_pcp_ping(&mut self, helo: &PcpHelo) -> Result<(), HandshakeError> {
        todo!()
    }

    /// pcp_rootだった場合の処理
    /// return : Remoteのポートが解放していればSome(port:u16), 解放されていなければNone
    async fn _incoming_pcp_root(&mut self, helo: &PcpHelo) -> Result<Option<u16>, HandshakeError> {
        // Is port opened ?
        let open_port_no: Option<u16> = check_port(
            &self.factory,
            self.inner.remote().ip(),
            helo.ping,
            helo.session_id,
        )
        .await;

        // return oleh
        let oleh_atom = OlehBuilder::new(
            self.inner.self_session_id,
            self.inner.remote.ip(),
            open_port_no.unwrap_or(0),
        )
        .build();
        self._write_atom(oleh_atom).await?;

        // return PCP_ROOT
        let root_atom = RootBuilder::new()
            .update_interval(120)
            .next_update_interval(120)
            .set_msg("PeerCast-RE ROOT SERVER".into())
            .set_root_update(false)
            .build();
        self._write_atom(root_atom).await?;

        // return ok
        let ok_atom = OkBuilder::new(0).build();
        self._write_atom(ok_atom).await?;

        // return first pcp_root
        let root_atom = RootBuilder::new().set_root_update(true).build();
        self._write_atom(root_atom).await?;

        Ok(open_port_no)
    }

    pub async fn outgoing_http(mut self) -> Result<PcpConnection, HandshakeError> {
        Ok(todo!())
    }
    pub async fn outgoing_pcp(mut self) -> Result<PcpConnection, HandshakeError> {
        // HelloBuilder::new(session_id, None);
        Ok(todo!())
    }

    // TODO: この関数、check_port()内に結合してもいいとおもう
    async fn outgoing_ping(&mut self) -> Result<PcpPong, HandshakeError> {
        let ping_atom = PingBuilder::new(self.inner.self_session_id).build();
        for a in ping_atom {
            let _ = self._write_atom(a).await;
        }
        let atom = self._read_atom().await?;
        let pong_info = PcpPong::parse(&atom)?;
        let quit_atom = self._read_atom().await?;
        Ok(pong_info)
    }

    #[inline]
    async fn _read_atom(&mut self) -> Result<Atom, std::io::Error> {
        self.inner.read_atom().await
    }

    #[inline]
    async fn _write_atom(&mut self, atom: Atom) -> Result<(), std::io::Error> {
        self.inner.write_atom(atom).await
    }
}

/// ポート解放チェックする関数
/// MEMO: PcpHandshakeに入れてもいいのでは？
///       でもIncomingで来た時しか使わないからなぁ
async fn check_port(
    factory: &PcpConnectionFactory,
    remote: IpAddr,
    port: Option<u16>,
    remote_session_id: GnuId,
) -> Option<u16> {
    let port = port?;
    let addr = (remote, port).into();
    let mut conn = factory.connect(addr).await.ok()?;
    let pong = conn.outgoing_ping().await.ok()?;

    if pong.session_id == remote_session_id {
        return Some(port);
    } else {
        return None;
    }
}

//--------------------------------------------------------------------------------
// PcpConnection
//
#[derive(Debug)]
pub enum PcpConnectType {
    Outgoing,
    IncomingPing(PcpPing),
    IncomingBroadcast(PcpHelo),
}

#[derive(Debug)]
pub struct PcpConnection {
    inner: Inner,
    pub remote_session_id: Arc<GnuId>,
    pub con_type: PcpConnectType,
    pub remote_port: Option<u16>,
}

impl PcpConnection {
    pub(super) fn new(
        inner: Inner,
        remote_session_id: GnuId,
        con_type: PcpConnectType,
        remote_port: Option<u16>,
    ) -> Self {
        Self {
            inner,
            remote_session_id: Arc::new(remote_session_id),
            con_type,
            remote_port,
        }
    }

    pub fn connection_id(&self) -> ConnectionId {
        self.inner.connection_id
    }

    pub async fn read_atom(&mut self) -> Result<Atom, std::io::Error> {
        self.inner.read_atom().await
    }
    pub async fn write_atom(&mut self, atom: Atom) -> Result<(), std::io::Error> {
        self.inner.write_atom(atom).await
    }

    pub fn split(self) -> (PcpConnectionReadHalf, PcpConnectionWriteHalf) {
        let Self {
            inner,
            remote_session_id,
            con_type,
            remote_port,
        } = self;

        let (read_half, write_half) = inner.split();
        let read_half = PcpConnectionReadHalf {
            inner: read_half,
            remote_session_id: Arc::clone(&remote_session_id),
            remote_port,
        };
        let write_half = PcpConnectionWriteHalf {
            inner: write_half,
            remote_session_id,
            remote_port,
        };
        (read_half, write_half)
    }
}

#[derive(Debug)]
pub struct PcpConnectionReadHalf {
    inner: InnerReadHalf,
    pub remote_session_id: Arc<GnuId>,
    // pub con_type: PcpConnectType,
    pub remote_port: Option<u16>,
}
impl PcpConnectionReadHalf {
    pub fn connection_id(&self) -> ConnectionId {
        self.inner.connection_id
    }
    pub async fn read_atom(&mut self) -> Result<Atom, std::io::Error> {
        self.inner.read_atom().await
    }
}

#[derive(Debug)]
pub struct PcpConnectionWriteHalf {
    inner: InnerWriteHalf,
    pub remote_session_id: Arc<GnuId>,
    // pub con_type: PcpConnectType,
    pub remote_port: Option<u16>,
}
impl PcpConnectionWriteHalf {
    pub fn connection_id(&self) -> ConnectionId {
        self.inner.connection_id
    }
    pub async fn write_atom(&mut self, atom: Atom) -> Result<(), std::io::Error> {
        self.inner.write_atom(atom).await
    }
}

#[cfg(test)]
mod t {
    use super::*;

    #[test]
    fn test_magic() {
        let mut head = [0_u8; 4];
        assert_eq!(PcpHandshake::PCP_MAGIC_HEAD == &head, false);
        head = *b"pcp\n";
        assert_eq!(PcpHandshake::PCP_MAGIC_HEAD == &head, true)
    }
}
