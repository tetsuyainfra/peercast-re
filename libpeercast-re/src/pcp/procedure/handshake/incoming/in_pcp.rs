use core::error;
use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};

use bytes::{Buf, BytesMut};
use futures_util::{stream, SinkExt, StreamExt};
use num::traits::real::Real;
use tokio::{io::AsyncReadExt, net::TcpStream, time::timeout};
use tokio_util::codec::{Framed, FramedParts};

use crate::{
    io::Io,
    net::IPVersion,
    pcp::{
        atom2::{Atom2Kind, AtomView},
        builder2::{HeloInfo, OlehBuilder2, OlehInfo},
        procedure::handshake::{HandshakeError, Parts, PartsWrapFramed},
        Atom, Atom2, AtomCodec, GnuId, Id4,
    },
    prelude::*,
    ConnectionNo,
};

const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Brief.
///
/// Description.
///
/// * `ip_version` - Text about foo.
/// * `in_type` - Text about bar.
pub struct ResultPcpHandshake<S: Io> {
    pub parts: PartsWrapFramed<S>,
    pub ip_version: IPVersion,
    pub in_type: IncommingType,
}

impl<S: Io> ResultPcpHandshake<S> {
    pub async fn shutdown(self) -> () {
        self.parts.shutdown().await
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum IncommingType {
    YellowPage(GnuId),
    Ping,
}

////////////////////////////////////////////////////////////////////////////////
// PCP
//
pub struct IncomingPcpHandshake<S: Io> {
    parts: Parts<S>,
    timeout: std::time::Duration,
}

impl<S: Io> IncomingPcpHandshake<S> {
    pub fn new(
        cno: ConnectionNo,
        stream: S,
        remote: SocketAddr,
        read_buf: Option<BytesMut>,
        write_buf: Option<BytesMut>,
    ) -> Self {
        Self {
            parts: Parts::new(cno, stream, remote, read_buf, write_buf),
            timeout: TIMEOUT.clone(),
        }
    }
    pub fn from_parts(parts: Parts<S>) -> Self {
        Self {
            parts,
            timeout: TIMEOUT.clone(),
        }
    }

    pub fn into_parts(self) -> Parts<S> {
        let Self {
            parts,
            timeout,
        } = self;
        parts
    }

    pub async fn shutdown(mut self) -> () {
        self.parts.stream.shutdown().await;
    }

    pub async fn handshake(self, self_session_id: GnuId) -> Result<ResultPcpHandshake<S>, HandshakeError> {
        let Self {
            parts:
                Parts {
                    cno,
                    stream,
                    remote,
                    read_buf,
                    write_buf,
                },
            timeout,
        } = self;

        // Framedを作成する
        let mut parts = FramedParts::new::<Atom2>(stream, AtomCodec::new());
        parts.read_buf = read_buf;
        let mut framed = Framed::from_parts(parts);

        // b"pcp\n\x04\x00\x00\x00" + "\x01\x00\x00\x00" を受信する
        let Some(Ok(magic)) = tokio::time::timeout(timeout, framed.next()).await? else {
            error!("failed to receive magic");
            return Err(HandshakeError::Failed);
        };
        if magic.id() != Id4::PCP_CONNECT {
            return Err(HandshakeError::Failed);
        }
        let ip_version = match magic.view() {
            Atom2Kind::Parent(parent_view) => {
                return Err(HandshakeError::Failed);
            }
            Atom2Kind::Child(child_view) => {
                let ver = child_view.data().try_get_u32_le().map_err(|_| HandshakeError::Failed)?;
                match ver {
                    1 => IPVersion::V4,
                    100 => IPVersion::V6,
                    _ => {
                        error!("invalid ip version");
                        return Err(HandshakeError::Failed);
                    }
                }
            }
        };
        dbg!(&ip_version);

        // heloの受信
        let helo = tokio::time::timeout(timeout, framed.next()).await?;
        dbg!(&helo);
        let Some(Ok(helo)) = helo else {
            return Err(HandshakeError::Failed);
        };
        if helo.id() != Id4::PCP_HELO {
            return Err(HandshakeError::Failed);
        }
        let helo_info = HeloInfo::try_from(&helo).map_err(|_| HandshakeError::Failed)?;
        let in_type = if let Some(broadcast_id) = helo_info.broadcast_id {
            // MEMO: BroadcastIDを送るのはYPのみだと思う
            IncommingType::YellowPage(broadcast_id)
        } else {
            IncommingType::Ping
        };
        dbg!(&helo_info);

        // olehの返信
        let remote_ip = remote.ip();
        let remote_port = 0; // HACKME, TODO
        let oleh = OlehBuilder2::new(self_session_id, remote_ip, remote_port).build();
        let helo = tokio::time::timeout(timeout, framed.send(oleh)).await?;

        let result = ResultPcpHandshake {
            parts: PartsWrapFramed {
                cno,
                remote,
                framed,
            },
            ip_version,
            in_type,
        };
        Ok(result)
    }
}
