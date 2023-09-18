use std::io;

use axum::extract::ConnectInfo;
use bytes::BytesMut;
use thiserror::Error;
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    net::TcpStream,
};
use tracing::log;

use crate::error::AtomParseError;

use super::Atom;

mod handshake;
mod http_req;
mod new_handshake;

pub use handshake::Handshake;

pub use new_handshake::{BothHandshake, HandshakeReturn};
