use core::error;
use std::net::SocketAddr;

use bytes::BytesMut;
use futures_util::{SinkExt, StreamExt};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tokio_util::codec::{Framed, FramedParts};
use tower_http::follow_redirect::policy::PolicyExt;
use tracing::debug;

use crate::pcp::{
    atom2::{codec::AtomCodec, Atom2},
    builder2::{OlehInfo, PingBuilder2},
    GnuId,
};

#[derive(Debug, thiserror::Error)]
pub enum HandshakeError {
    #[error("failed")]
    Failed,

    #[error(transparent)]
    AtomParseError(#[from] crate::error::AtomParseError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub struct OutgoingPcpHandshake {
    stream: TcpStream,
    remote: SocketAddr,
    read_buf: BytesMut,
}

impl OutgoingPcpHandshake {
    pub fn new(stream: TcpStream, remote: SocketAddr, read_buf: Option<BytesMut>) -> Self {
        Self {
            stream,
            remote,
            read_buf: read_buf.unwrap_or_else(|| BytesMut::with_capacity(4096)),
        }
    }

    pub fn finish(self) -> (TcpStream, SocketAddr, BytesMut) {
        let Self {
            stream,
            remote,
            read_buf,
        } = self;

        (stream, remote, read_buf)
    }

    pub async fn is_ping(
        mut self,
        self_session_id: GnuId,
        remote_session_id: GnuId,
        port: Option<u16>,
        port_check: Option<u16>,
    ) -> bool {
        let Ok((oleh, _)) = self.ping(self_session_id, port, port_check).await else {
            return false;
        };
        return oleh.session_id == remote_session_id;
    }

    pub async fn ping(
        mut self,
        self_session_id: GnuId,
        // remote_session_id: Option<GnuId>,
        port: Option<u16>,
        port_check: Option<u16>,
    ) -> Result<(OlehInfo, HandshakeResult), HandshakeError> {
        let Self {
            stream,
            remote,
            read_buf,
        } = self;
        let atoms = PingBuilder2::new(self_session_id).port(port).port_check(port_check).build();

        let mut framed = Framed::new(stream, AtomCodec::new());
        for a in atoms {
            dbg!("send:  {:?}", &a);
            framed.send(a).await?;
        }

        let mut atom = framed.next().await;
        let oleh_candidate = match atom {
            None => todo!(),
            Some(Err(e)) => todo!(),
            Some(Ok(a)) => a,
        };

        let oleh = OlehInfo::try_from(oleh_candidate).map_err(|_| HandshakeError::Failed)?;

        let FramedParts {
            io,
            codec,
            read_buf,
            write_buf,
            ..
        } = framed.into_parts();

        let ret = HandshakeResult {
            stream: io,
            remote: remote,
            read_buf: read_buf,
            write_buf: write_buf,
        };

        Ok((oleh, ret))
    }
}

#[derive(Debug)]
pub struct HandshakeResult {
    pub stream: TcpStream,
    pub remote: SocketAddr,
    pub read_buf: BytesMut,
    pub write_buf: BytesMut,
}
