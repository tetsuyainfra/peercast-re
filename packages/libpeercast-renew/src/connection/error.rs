use thiserror::Error;

use crate::{atom::error::AtomParseError, pcp::builder::error::InfoParseError};

// 主に通信について
#[derive(Debug, Error)]
pub enum ConnectionError {
    #[error("Handshake failed")]
    Handshake(#[from] HandshakeError),
}

// 主に通信について
#[derive(Debug, Error)]
pub enum HandshakeError {
    #[error("HttpResponse")]
    HttpResponse,

    #[error("ChannelNotFound")]
    ChannelNotFound,

    #[error("Could not find a server to connect")]
    ServerNotFound,

    #[error("Timeout")]
    Timeout,

    #[error("Connection Closed")]
    ConnectionClosed,

    #[error("AtomParsing error")]
    AtomParse(#[from] AtomParseError),

    #[error("InfoParsing error")]
    InfoParse(#[from] InfoParseError),

    #[error("io error")]
    IoError(#[from] std::io::Error),

    #[error("failed")]
    Failed,
}
