use std::net::SocketAddr;

use thiserror::Error;

mod factory;
mod inner;
mod pcp;

#[derive(Error, Debug, PartialEq, Clone)]
pub enum PcpError {
    #[error("failed handshake")]
    FailedHandshake,
}

// 接続の方向を表す
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionType {
    Client,
    Server,
}

#[derive(Debug, Clone)]
struct ConnectionInfo {
    remote: SocketAddr,
    connection_type: ConnectionType,
}
// imports
pub use factory::PcpConnectionFactory;

// pcp connections
pub use pcp::HandshakeType;
pub use pcp::PcpConnection;
