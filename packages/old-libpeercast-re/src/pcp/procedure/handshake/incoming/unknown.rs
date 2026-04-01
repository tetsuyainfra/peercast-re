use std::net::SocketAddr;

use bytes::BytesMut;

use crate::{io::Io, pcp::procedure::handshake::Parts, ConnectionNo};

////////////////////////////////////////////////////////////////////////////////
// Unknown
//
pub struct IncomingUnknownHandshake<S: Io> {
    pub parts: Parts<S>,
}

impl<S: Io> IncomingUnknownHandshake<S> {
    pub fn new(
        cno: ConnectionNo,
        stream: S,
        remote: SocketAddr,
        read_buf: Option<BytesMut>,
        write_buf: Option<BytesMut>,
    ) -> Self {
        Self {
            parts: Parts::new(cno, stream, remote, read_buf, None),
        }
    }

    pub fn from_parts(parts: Parts<S>) -> Self {
        Self {
            parts,
        }
    }
    pub fn into_parts(self) -> Parts<S> {
        let Self {
            parts,
        } = self;
        parts
    }

    pub async fn shutdown(mut self) -> () {
        self.parts.stream.shutdown().await;
    }
}
