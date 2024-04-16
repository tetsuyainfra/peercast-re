use std::{net::AddrParseError, num::ParseIntError, str::ParseBoolError};

use thiserror::Error;

use crate::pcp::{GnuId, GnuIdParseError};

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

    #[error("Parsing error {0}")]
    Parse(#[from] AtomParseError),

    #[error("io error")]
    IoError(#[from] std::io::Error),

    #[error("failed")]
    Failed,
}

// 主にデータ解析について
#[derive(Error, Debug)]
pub enum AtomParseError {
    // 少なくとも処理を進めるために必要なバイト数(このバイト数があればパースできるとは限らない)
    #[error("at least, recieve {0} bytes. but not enough for the atom.")]
    NotEnoughRecievedBuffer(usize),

    // 少なくとも処理を進めるために必要なバイト数(このバイト数があればパースできるとは限らない)
    #[error("Not found need value")]
    NotFoundValue,

    #[error("invalid atom id")]
    IdError,

    #[error("invalid atom value")]
    ValueError,

    #[error("unknown parse error")]
    Unknown,
}

// 主にConfigについて
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IoError {0:?}")]
    Io(#[from] std::io::Error),

    #[error("Syntax error. {0}")]
    Syntax(#[from] ini::ParseError),

    #[error(transparent)]
    ParseVariable(#[from] ParseVariableError),
}
#[derive(Error, Debug)]
pub enum ParseVariableError {
    // Integer(IntPars),
    #[error("parse vaiable error. error occured: {0}")]
    Ip(#[from] AddrParseError),

    #[error("parse vaiable error. error occured: {0}")]
    Integer(#[from] ParseIntError),

    #[error("parse vaiable error. error occured: {0}")]
    Bool(#[from] ParseBoolError),

    #[error("parse vaiable error. error occured: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("parse vaiable error. error occured: {0}")]
    GnuId(#[from] GnuIdParseError),
}

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Password is not matche to store password hash.")]
    WrongPassword,
}
