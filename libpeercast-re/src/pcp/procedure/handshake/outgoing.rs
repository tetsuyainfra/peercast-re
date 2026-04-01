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

use super::HandshakeError;
use crate::{
    io::Io,
    pcp::{
        atom2::{codec::AtomCodec, Atom2},
        builder2::{OlehInfo, PingBuilder2},
        procedure::handshake::PartsWrapFramed,
        GnuId,
    },
    ConnectionNo,
};

pub struct OutgoingPcpHandshake<S: Io> {
    cno: ConnectionNo,
    stream: S,
    remote: SocketAddr,
    read_buf: BytesMut,
}

impl<S: Io> OutgoingPcpHandshake<S> {
    pub fn new(stream: S, remote: SocketAddr, read_buf: Option<BytesMut>) -> Self {
        Self {
            cno: ConnectionNo::new(),
            stream,
            remote,
            read_buf: read_buf.unwrap_or_else(|| BytesMut::with_capacity(4096)),
        }
    }

    pub fn finish(self) -> (S, SocketAddr, BytesMut) {
        let Self {
            stream,
            remote,
            read_buf,
            cno,
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
    ) -> Result<(OlehInfo, HandshakeResult<S>), HandshakeError> {
        let Self {
            cno,
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

        let oleh = OlehInfo::try_from(&oleh_candidate).map_err(|_| HandshakeError::Failed)?;

        let parts = PartsWrapFramed {
            cno,
            remote,
            framed,
        };
        let ret = HandshakeResult {
            parts,
        };

        Ok((oleh, ret))
    }
}

#[derive(Debug)]
pub struct HandshakeResult<S: Io> {
    pub parts: PartsWrapFramed<S>,
}
