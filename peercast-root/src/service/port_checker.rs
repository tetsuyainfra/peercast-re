use std::net::{IpAddr, SocketAddr};

use libpeercast_re::pcp::connection5::{ConnectionFactory, OutgoingConnection, ping::Ping};

use crate::{
    connection::{RootConnectionFactory, RootSpec},
    model::PortLevel,
};

#[async_trait::async_trait]
pub trait PortChecker: Send + Sync {
    /// ポートチェック
    /// - target_addr: チェック対象のIPアドレス
    /// - target_port: チェック対象のポート番号
    /// - 戻り値: PortLevel
    async fn check(&self, addr: IpAddr, port: u16) -> PortLevel;
}

pub struct PingPortChecker<'a> {
    conn_factory: &'a RootConnectionFactory,
}

impl<'a> PingPortChecker<'a> {
    pub fn new(conn_factory: &'a RootConnectionFactory) -> Self {
        Self {
            conn_factory,
        }
    }
}

#[async_trait::async_trait]
impl<'a> PortChecker for PingPortChecker<'a> {
    async fn check(&self, addr: IpAddr, port: u16) -> PortLevel {
        let remote = SocketAddr::new(addr, port);
        let conn_ping = self.conn_factory.create_outgoing::<Ping<RootSpec>>(remote, None);

        if conn_ping.connect().await.is_ok() {
            PortLevel::Welldone
        } else {
            PortLevel::Incomplete
        }
    }
}

#[cfg(test)]
pub struct MockPortChecker {
    pub result: PortLevel,
}

#[cfg(test)]
#[async_trait::async_trait]
impl PortChecker for MockPortChecker {
    async fn check(&self, _addr: IpAddr, _port: u16) -> PortLevel {
        self.result
    }
}
