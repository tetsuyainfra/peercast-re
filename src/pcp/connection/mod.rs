mod pcp_connection;

#[derive(Debug, Error)]
pub enum PcpError {}

use std::{
    collections::{HashMap, VecDeque},
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use bytes::BytesMut;
use thiserror::Error;
use tokio::{net::TcpStream, sync::watch};
use tracing::info;

use crate::{
    error::{self, HandshakeError},
    pcp::atom,
    ConnectionId,
};

use self::pcp_connection::Inner;
pub use self::pcp_connection::{
    PcpConnectType, PcpConnection, PcpConnectionReadHalf, PcpConnectionWriteHalf, PcpHandshake,
};

use super::{Atom, GnuId};
//--------------------------------------------------------------------------------
// FactoryConfig
//
#[derive(Debug)]
pub struct FactoryConfig {
    connect_timeout: Duration,
    disconnection_timeout: Duration,
}
impl Default for FactoryConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(5),        // 5sec
            disconnection_timeout: Duration::from_secs(15), // 15sec
        }
    }
}

//--------------------------------------------------------------------------------
// FactoryBuilder
//
pub struct FactoryBuilder {
    config: Option<FactoryConfig>,
    self_session_id: GnuId,
}

impl FactoryBuilder {
    fn new(self_session_id: GnuId) -> Self {
        Self {
            config: None,
            self_session_id,
        }
    }
    pub fn build(self) -> PcpConnectionFactory {
        PcpConnectionFactory {
            impl_: Arc::new(PcpConnectionImpl::new(
                self.config.unwrap_or_default(),
                self.self_session_id,
            )),
        }
    }
}

//--------------------------------------------------------------------------------
// PcpConnectionImpl
//
#[derive(Debug)]
pub struct PcpConnectionImpl {
    config: FactoryConfig,
    self_session_id: GnuId,
}

impl PcpConnectionImpl {
    pub fn new(config: FactoryConfig, self_session_id: GnuId) -> Self {
        Self {
            config,
            self_session_id,
        }
    }
}

//--------------------------------------------------------------------------------
// PcpConnectionFactory
//
#[derive(Debug)]
pub struct PcpConnectionFactory {
    impl_: Arc<PcpConnectionImpl>,
}
impl Clone for PcpConnectionFactory {
    fn clone(&self) -> Self {
        Self {
            impl_: Arc::clone(&self.impl_),
        }
    }
}

impl PcpConnectionFactory {
    pub fn new(self_session_id: GnuId) -> Self {
        Self::builder(self_session_id).build()
    }
    pub fn builder(self_session_id: GnuId) -> FactoryBuilder {
        FactoryBuilder::new(self_session_id)
    }

    // async fn _connect(&self, addr: SocketAddr) -> Result<PcpConnection, HandshakeError> {
    //     let conn = tokio::time::timeout(self.config.connect_timeout, TcpStream::connect(addr))
    //         .await
    //         .map_err(|_e| HandshakeError::ServerNotFound)??;

    //     let connection_id = ConnectionId::new();
    //     let pcp_hand =
    //         PcpConnectionInner::new(connection_id.clone(), self_session_id, conn, addr, None);

    //     todo!()
    // }
    pub async fn connect(&self, remote_addr: SocketAddr) -> Result<PcpHandshake, std::io::Error> {
        let stream = TcpStream::connect(remote_addr).await?;
        let inner = pcp_connection::Inner::new(
            ConnectionId::new(),
            self.impl_.self_session_id,
            stream,
            remote_addr,
            None,
        );
        info!(
            "PCP ACCEPT CID:{} REMOTE:{}",
            inner.connection_id(),
            inner.remote()
        );
        Ok(PcpHandshake::new(inner, self.clone()))
    }

    pub fn accept(&self, stream: TcpStream, remote: SocketAddr) -> PcpHandshake {
        let inner = pcp_connection::Inner::new(
            ConnectionId::new(),
            self.impl_.self_session_id,
            stream,
            remote,
            None,
        );
        info!(
            "PCP ACCEPT CID:{} REMOTE:{}",
            inner.connection_id(),
            inner.remote()
        );
        PcpHandshake::new(inner, self.clone())
    }
}

#[cfg(test)]
mod t {
    use super::*;

    #[allow(dead_code)]
    async fn temp() {
        let self_session_id = GnuId::new();
        let factory = PcpConnectionFactory::new(self_session_id);
    }
}
