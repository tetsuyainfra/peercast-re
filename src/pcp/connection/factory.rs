use std::{
    collections::{HashMap, VecDeque},
    net::SocketAddr,
    sync::{Arc, Mutex, RwLock},
    time::{Duration, Instant},
};

use bytes::BytesMut;
use thiserror::Error;
use tokio::{net::TcpStream, sync::watch};
use tracing::info;

use crate::{
    error::{self, HandshakeError},
    pcp::{
        atom,
        connection::{pcp::PcpHandshake, ConnectionType},
        GnuId,
    },
    util::{mutex_poisoned, rwlock_read_poisoned, rwlock_write_poisoned},
    ConnectionId,
};

use super::ConnectionInfo;

//--------------------------------------------------------------------------------
// ConnectionManager for PcpConnectio
//

#[derive(Debug)]
pub struct ConnectionManager {
    connections: HashMap<ConnectionId, ConnectionInfo>,
}

impl ConnectionManager {
    fn new() -> Self {
        Self {
            connections: Default::default(),
        }
    }

    pub fn register_handshake(&mut self, handshake: &PcpHandshake) {
        let connection_info = ConnectionInfo {
            remote: handshake.remote_addr().clone(),
            connection_type: handshake.connection_type(),
        };

        self.connections
            .insert(handshake.connection_id(), connection_info);
    }
}
//--------------------------------------------------------------------------------
// FactoryImpl for PcpConnection
//
#[derive(Debug)]
pub struct FactoryImpl {
    self_session_id: GnuId,
    config: FactoryConfig,
    manager: Mutex<ConnectionManager>,
}

impl FactoryImpl {
    pub fn new(self_session_id: GnuId, config: FactoryConfig) -> Self {
        Self {
            self_session_id,
            config,
            manager: Mutex::new(ConnectionManager::new()),
        }
    }

    pub async fn connect(
        &self,
        remote_addr: SocketAddr,
        factory: &PcpConnectionFactory,
    ) -> Result<PcpHandshake, std::io::Error> {
        let stream =
            tokio::time::timeout(self.config.connect_timeout, TcpStream::connect(remote_addr))
                .await??;

        let cid = ConnectionId::new();
        let handshake = PcpHandshake::new(
            cid,
            self.self_session_id,
            stream,
            remote_addr,
            ConnectionType::Client,
            None,
        );
        self.manager
            .lock()
            .unwrap_or_else(mutex_poisoned)
            .register_handshake(&handshake);

        info!("PCP OUTGOING ACCEPT({})", cid);
        Ok(handshake)
    }

    pub fn accept(
        &self,
        cid: ConnectionId,
        stream: TcpStream,
        remote_addr: SocketAddr,
        factory: &PcpConnectionFactory,
    ) -> PcpHandshake {
        let handshake = PcpHandshake::new(
            cid,
            self.self_session_id,
            stream,
            remote_addr,
            ConnectionType::Server,
            None,
        );
        self.manager
            .lock()
            .unwrap_or_else(mutex_poisoned)
            .register_handshake(&handshake);

        info!("PCP INCMOING ACCEPT({})", cid);
        handshake
    }
}

//--------------------------------------------------------------------------------
// PcpConnectionFactory
//
#[derive(Debug)]
pub struct PcpConnectionFactory {
    impl_: Arc<FactoryImpl>,
}
impl Clone for PcpConnectionFactory {
    fn clone(&self) -> Self {
        Self {
            impl_: Arc::clone(&self.impl_),
        }
    }
}

impl PcpConnectionFactory {
    // デフォルト設定でFactoryを作成
    pub fn new(self_session_id: GnuId, self_socket: SocketAddr) -> Self {
        Self::builder(self_session_id, self_socket).build()
    }
    // Factoryの詳細を設定したい場合
    pub fn builder(self_session_id: GnuId, self_socket: SocketAddr) -> FactoryBuilder {
        FactoryBuilder::new(self_session_id, self_socket)
    }

    pub async fn connect(&self, remote_addr: SocketAddr) -> Result<PcpHandshake, std::io::Error> {
        self.impl_.connect(remote_addr, &self).await
    }
    pub fn accept(&self, cid: ConnectionId, stream: TcpStream, remote: SocketAddr) -> PcpHandshake {
        self.impl_.accept(cid, stream, remote, &self)
    }
}

//--------------------------------------------------------------------------------
// FactoryConfig
//
#[derive(Debug)]
pub struct FactoryConfig {
    // 接続(Outgoing)時のタイムアウト時間(default: 5sec)
    connect_timeout: Duration,
    // Handshake終了までのタイムアウト時間(default: 2sec)
    handshake_timeout: Duration,
    // 接続後のタイムアウト時間(default: 15sec)
    disconnection_timeout: Duration,
}
impl Default for FactoryConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(5),        // 5sec
            handshake_timeout: Duration::from_secs(2),      // 2sec
            disconnection_timeout: Duration::from_secs(15), // 15sec
        }
    }
}

//--------------------------------------------------------------------------------
// FactoryBuilder
//
pub struct FactoryBuilder {
    config: FactoryConfig,
    self_session_id: GnuId,
    self_socket: SocketAddr,
}

impl FactoryBuilder {
    fn new(self_session_id: GnuId, self_socket: SocketAddr) -> Self {
        Self {
            config: Default::default(),
            self_session_id,
            self_socket,
        }
    }
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.config.connect_timeout = timeout;
        self
    }

    pub fn build(self) -> PcpConnectionFactory {
        let Self {
            config,
            self_session_id,
            self_socket,
        } = self;
        PcpConnectionFactory {
            impl_: Arc::new(FactoryImpl::new(self_session_id, config)),
        }
    }
}

#[cfg(test)]
mod t {
    use crate::test_helper::*;

    use super::*;

    #[crate::test]
    async fn test_fundamental() {
        assert_send::<FactoryImpl>();
        assert_sync::<FactoryImpl>();
        assert_send::<PcpConnectionFactory>();
        assert_sync::<PcpConnectionFactory>();
        let self_session_id = GnuId::new();
        let self_socket = "192.168.0.1".parse().unwrap();
        let factory = PcpConnectionFactory::new(self_session_id, self_socket);
    }
}
