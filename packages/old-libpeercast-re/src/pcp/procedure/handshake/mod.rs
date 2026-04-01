mod discriminator;
mod incoming;
mod outgoing;

use std::os::unix::net::SocketAddr;

pub use discriminator::*;
pub use incoming::*;
pub use outgoing::*;
use tokio_util::codec::Framed;

#[derive(Debug, thiserror::Error)]
pub enum HandshakeError {
    #[error("failed")]
    Failed,

    #[error("Identify failed")]
    ProtocolIdentifyFailed,

    #[error(transparent)]
    AtomParseError(#[from] crate::error::AtomParseError),

    #[error(transparent)]
    Timeout(#[from] tokio::time::error::Elapsed),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug)]
pub struct Parts<S: crate::io::Io> {
    pub cno: crate::ConnectionNo,
    pub stream: S,
    pub remote: std::net::SocketAddr,
    pub read_buf: bytes::BytesMut,
    pub write_buf: bytes::BytesMut,
}

impl<S: crate::io::Io> Parts<S> {
    fn new(
        cno: crate::ConnectionNo,
        stream: S,
        remote: std::net::SocketAddr,
        read_buf: Option<bytes::BytesMut>,
        write_buf: Option<bytes::BytesMut>,
    ) -> Self {
        Self {
            cno,
            stream,
            remote,
            read_buf: read_buf
                .unwrap_or_else(|| bytes::BytesMut::with_capacity(crate::DEFAULT_HTTP_HEADER_ACCEPT_SIZE)),
            write_buf: write_buf
                .unwrap_or_else(|| bytes::BytesMut::with_capacity(crate::DEFAULT_HTTP_HEADER_ACCEPT_SIZE)),
        }
    }

    pub async fn shutdown(mut self) -> () {
        self.stream.shutdown().await;
    }
}

/// tokio_util::codec::Framedのラッパー
#[derive(Debug)]
pub struct PartsWrapFramed<S: crate::io::Io> {
    pub cno: crate::ConnectionNo,
    pub remote: std::net::SocketAddr,
    pub framed: Framed<S, crate::pcp::AtomCodec>,
}

impl<S: crate::io::Io> PartsWrapFramed<S> {
    fn new(cno: crate::ConnectionNo, remote: std::net::SocketAddr, framed: Framed<S, crate::pcp::AtomCodec>) -> Self {
        Self {
            cno,
            remote,
            framed,
        }
    }

    pub async fn shutdown(mut self) -> () {
        self.framed.into_inner().shutdown().await;
    }
}
