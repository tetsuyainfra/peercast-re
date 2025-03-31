// SPDX-FileCopyrightText: 2025 tetsuyainfra
// SPDX-License-Identifier: MIT
use std::{
    collections::VecDeque,
    future::{Future, IntoFuture},
    net::SocketAddr,
    time::Instant,
};

use bytes::BytesMut;
use chrono::format::parse;
use futures_util::FutureExt;
use tokio::net::TcpStream;

use crate::{
    pcp::{
        atom,
        builder::{
            HelloBuilder, OkBuilder, OlehBuilder, OlehInfo, PingBuilder, PongBuilder, QuitBuilder,
        },
        decode::{decode_i32, PcpBroadcast, PcpHelo},
        Atom, GnuId, Id4,
    },
    ConnectionId,
};

use super::{
    inner::{Inner, ReadHalfInner, WriteHalfInner},
    ConnectionInfo, ConnectionType, PcpError,
};

////////////////////////////////////////////////////////////////////////////////
///  PcpHandshak
///
#[derive(Debug)]
pub struct PcpHandshake {
    inner: Inner,
    connection_type: ConnectionType,
}
impl PcpHandshake {
    pub(super) fn new(
        cid: ConnectionId,
        self_session_id: GnuId,
        stream: TcpStream,
        socket_addr: SocketAddr,
        connection_type: ConnectionType,
        read_buf: Option<BytesMut>,
    ) -> Self {
        let inner = Inner::new(cid, self_session_id, stream, socket_addr, read_buf);
        Self {
            inner,
            connection_type,
        }
    }

    pub fn connection_id(&self) -> ConnectionId {
        self.inner.connection_id()
    }
    pub fn remote_addr(&self) -> &SocketAddr {
        self.inner.remote_addr()
    }
    pub fn connection_type(&self) -> ConnectionType {
        self.connection_type
    }

    /// pingを打つのみ
    pub fn ping(self) -> PingPongFuture {
        let Self {
            mut inner,
            connection_type,
        } = self;

        async move {
            let mut ping = PingBuilder::new(inner.self_session_id().clone()).build();

            if let Err(_e) = inner.write_atoms(&mut ping).await {
                return Err(todo!());
            }

            println!("sendok");
            let oleh = match inner.read_atom().await {
                Ok(a) => a,
                Err(_e) => return Err(todo!()),
            };

            println!("readokok");
            let oleh = OlehInfo::parse(&oleh);
            println!("{:#?}", oleh);

            Ok(oleh.session_id)
        }
        .boxed()
    }

    /// pingを打つかつ、相手に自分のポートチェックをしてもらう
    pub fn ping_with_portcheck(self, port: u16) -> PingPongFuture {
        let PcpHandshake {
            mut inner,
            connection_type,
        } = self;

        async move {
            // FIXME: OpenPortとPingPortの関係がイマイチわからん！
            let mut ping = PingBuilder::new(inner.self_session_id().clone())
                .port(Some(port))
                .port_check(Some(port))
                .build();
            dbg!(&ping);
            if let Err(_e) = inner.write_atoms(&mut ping).await {
                return Err(todo!());
            }

            let oleh = match inner.read_atom().await {
                Ok(a) => a,
                Err(e) => {
                    dbg!(&e);
                    return Err(e);
                }
            };
            let oleh = OlehInfo::parse(&oleh);
            println!("{:#?}", oleh);

            Ok(oleh.session_id)
        }
        .boxed()
    }

    /// 接続の待ち受け
    pub async fn incoming(
        self,
        send_atom_before_ok: Option<Atom>,
    ) -> Result<HandshakeType, PcpError> {
        let PcpHandshake {
            mut inner,
            connection_type,
        } = self;
        assert_eq!(ConnectionType::Server, connection_type);

        // Protocol Check
        let magic = inner
            .read_atom()
            .await
            .map_err(|_| PcpError::FailedHandshake)?;
        if magic.is_parent() {
            return Err(PcpError::FailedHandshake);
        }
        let magic = magic.as_child();
        if magic.id() != Id4::PCP_CONNECT {
            return Err(PcpError::FailedHandshake);
        }
        if magic.len() != 4 {
            return Err(PcpError::FailedHandshake);
        }
        let r = decode_i32(magic).map_err(|_| PcpError::FailedHandshake)?;
        if 1 != r {
            return Err(PcpError::FailedHandshake);
        }

        let helo = inner
            .read_atom()
            .await
            .map_err(|_| PcpError::FailedHandshake)?;
        dbg!(&helo);

        // Pingか判定する(なんだかんだスレッド作るのはコスト高いため)
        if (helo.is_parent() && helo.len() == 1) {
            let oleh = PongBuilder::new(inner.self_session_id().clone()).build();
            let quit = QuitBuilder::new(crate::pcp::builder::QuitReason::Any).build();
            inner
                .write_atoms(&mut VecDeque::from(vec![oleh, quit]))
                .await
                .map_err(|_| PcpError::FailedHandshake)?;
            inner.shutdown();
            return Ok(HandshakeType::Ping);
        }

        // Helo
        let helo = PcpHelo::parse(&helo).map_err(|_| PcpError::FailedHandshake)?;
        let remote_ip = inner.remote_addr().clone().ip();
        let remote_port = helo.port.ok_or(PcpError::FailedHandshake)?;

        let oleh =
            OlehBuilder::new(inner.self_session_id().clone(), remote_ip, remote_port).build();
        let ok = OkBuilder::new(1).build();

        let atoms = match send_atom_before_ok {
            Some(a) => vec![oleh, a, ok],
            None => vec![oleh, ok],
        };

        // SEND OLEH & (ROOT) & PCP_OK
        inner
            .write_atoms(&mut VecDeque::from(atoms))
            .await
            .map_err(|_| PcpError::FailedHandshake)?;

        let conn = PcpConnection::new(inner, connection_type);
        Ok(HandshakeType::YellowPage(conn))
    }
}

type PingPongFuture = futures_util::future::BoxFuture<'static, Result<GnuId, std::io::Error>>;
type ConnectionFuture =
    futures_util::future::BoxFuture<'static, Result<HandshakeType, std::io::Error>>;

pub enum HandshakeType {
    Ping,
    YellowPage(PcpConnection),
}

////////////////////////////////////////////////////////////////////////////////
///  PcpConnection
///
#[derive(Debug)]
pub struct PcpConnection {
    inner: Inner,
    connection_type: ConnectionType,
}

impl PcpConnection {
    fn new(inner: Inner, connection_type: ConnectionType) -> Self {
        Self {
            inner,
            connection_type,
        }
    }

    #[inline]
    pub fn remote_addr(&self) -> SocketAddr {
        self.inner.remote_addr().clone()
    }

    #[inline]
    pub async fn read_atom(&mut self) -> Result<Atom, std::io::Error> {
        self.inner.read_atom().await
    }

    #[inline]
    pub async fn write_atom(&mut self, atom: Atom) -> Result<(), std::io::Error> {
        self.inner.write_atom(atom).await
    }

    pub fn split(mut self) -> (ReadHalfInner, WriteHalfInner) {
        self.inner.split()
    }
}

#[cfg(test)]
mod t {
    use crate::{
        pcp::{
            connection::{factory, pcp::PcpHandshake, ConnectionType},
            GnuId, PcpConnectionFactory,
        },
        ConnectionId,
    };

    #[tokio::test]
    async fn test_outgoing() {
        let addr = todo!();
        let stream = todo!();
        let connection = PcpHandshake::new(
            ConnectionId::new(),
            GnuId::new(),
            stream,
            addr,
            ConnectionType::Client,
            None,
        );
    }
}
