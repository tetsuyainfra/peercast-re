use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};

use peercast_atom::{codec::AtomCodec, error::AtomCodecError};
use peercast_gnuid::GnuId;
use tokio::net::{TcpListener, TcpSocket};
use tokio_util::{codec::Framed, net::Listener};

use crate::pcp::builder::{MagicBuilder, OlehInfo, PingBuilder};

#[derive(Debug, thiserror::Error)]
pub enum PortCheckerError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Atom codec error")]
    AtomCodecError(#[from] AtomCodecError),

    #[error("Atom info parse error")]
    InfoParseError(#[from] crate::pcp::builder::error::InfoParseError),

    #[error("Timeout error")]
    TimeoutError,
}

#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait::async_trait]
pub trait PortChecker: Send + Sync {
    /// ポートチェック
    /// - target_addr: チェック対象のIPアドレス
    /// - target_port: チェック対象のポート番号
    /// - 戻り値: Result<true:ポートが開いている, false:ポートが閉じている, Err:エラー>
    async fn check(&self, remote_addr: IpAddr, remote_port: u16) -> Result<(), PortCheckerError>;
    async fn check_with_session_id(
        &self,
        remote_addr: IpAddr,
        remote_port: u16,
        remote_session_id: GnuId,
    ) -> Result<bool, PortCheckerError>;
}

#[derive(Debug)]
pub struct PortCheckerService {
    self_session_id: GnuId,
    timeout: Duration,
}

impl PortCheckerService {
    pub fn new(self_session_id: GnuId, timeout: Duration) -> Self {
        Self {
            self_session_id,
            timeout,
        }
    }

    async fn _check(&self, remote_addr: IpAddr, remote_port: u16) -> Result<OlehInfo, PortCheckerError> {
        use futures::{SinkExt, StreamExt};

        if remote_addr.is_ipv4() {
            // IPv4アドレスの場合はIPv4ソケットを使用
            let socket = TcpSocket::new_v4()?;
            let listener = socket.connect(SocketAddr::new(remote_addr, remote_port)).await?;
            let mut framed = Framed::new(listener, AtomCodec::default());

            let magic = MagicBuilder::new(crate::model::IpMode::IpV4).build();
            let _ = framed.send(magic).await?;

            let helo = PingBuilder::new(self.self_session_id).build();
            let _ = framed.send(helo).await?;

            let oleh = framed.next().await.ok_or_else(|| {
                PortCheckerError::IoError(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Connection closed"))
            })??;

            let oleh_info = OlehInfo::try_from(&oleh)?;

            Ok(oleh_info)
        } else {
            // IPv6アドレスの場合はIPv6ソケットを使用
            panic!("IPv6 is not supported yet");
        }
    }

    async fn _check_with_timeout(&self, remote_addr: IpAddr, remote_port: u16) -> Result<OlehInfo, PortCheckerError> {
        tokio::time::timeout(self.timeout, self._check(remote_addr, remote_port))
            .await
            .map_err(|_| PortCheckerError::TimeoutError)?
    }
}

#[async_trait::async_trait]
impl PortChecker for PortCheckerService {
    async fn check(&self, remote_addr: IpAddr, remote_port: u16) -> Result<(), PortCheckerError> {
        self._check_with_timeout(remote_addr, remote_port).await.map(|_| ())
    }

    async fn check_with_session_id(
        &self,
        remote_addr: IpAddr,
        remote_port: u16,
        remote_session_id: GnuId,
    ) -> Result<bool, PortCheckerError> {
        let oleh_info = self._check_with_timeout(remote_addr, remote_port).await?;
        Ok(oleh_info.session_id == Some(remote_session_id))
    }
}

#[cfg(test)]
mod t {
    use super::*;

    #[tokio::test]
    async fn test_check() {
        // テストコードをここに追加
    }
}
