use std::{
    sync::{atomic::AtomicU64, Arc, Weak},
    time::Instant,
};

use crate::ConnectionNo;

use bytes::BytesMut;
use hyper_util::client::legacy::connect::Connect;
use tokio::io::{AsyncRead, AsyncWrite};

pub trait Io: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Io for T {}

//コネクションを管理するモジュール
pub trait ConnectionManager<Handle: ConnectionHandle>: Send + Sync {
    fn insert(&self, handle: Handle);
    fn remove(&self, cno: &ConnectionNo) -> Option<Handle>;

    fn report_metrics(&self) -> Vec<(ConnectionNo, Arc<ConnectionStats>)>;
}

///コネクションを外部から操作するためのハンドル
pub trait ConnectionHandle {
    fn cno(&self) -> ConnectionNo;
    fn stats(&self) -> Arc<ConnectionStats>;
}

/// コネクション本体
/// Dropトレイトを実装する必要があり、そこでは管理されているConnectionManagerからコネクションを削除してもらう必要がある
#[async_trait::async_trait]
pub trait Connection: Sized {
    type Handle: ConnectionHandle;
    type Manager: ConnectionManager<Self::Handle>;

    fn cno(&self) -> ConnectionNo;
    fn handle(&self) -> Self::Handle;
    fn stats(&self) -> Arc<ConnectionStats>;
    /// このコネクションを管理するマネージャーの弱参照を返す
    fn owner(&self) -> &Weak<Self::Manager>;

    /// このトレイトを実装する場合、Dropトレイトを実装し、このメソッド呼び出すこと
    fn on_drop(&self) {
        if let Some(manager) = self.owner().upgrade() {
            manager.remove(&self.cno());
        }
    }

    async fn run(self);
}

/// コネクションが持つ統計情報
#[derive(Debug)]
pub struct ConnectionStats {
    pub start_time: Instant,
    pub bytes_read: AtomicU64,
    pub bytes_written: AtomicU64,
}

impl ConnectionStats {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            bytes_read: 0.into(),
            bytes_written: 0.into(),
        }
    }
}

/// コネクションを生成するためのファクトリ
pub trait ConnectionFactory {
    type Conn: Connection;
    type Handle: ConnectionHandle;
    type Manager: ConnectionManager<Self::Handle>;

    fn manager(&self) -> &Self::Manager;

    fn create_accepted_connection<S: Io>(
        &self,
        cno: ConnectionNo,
        stream: S,
        remote: std::net::SocketAddr,
        write_buf: BytesMut,
        read_buf: BytesMut,
    ) -> Self::Conn;

    fn create_outgoing_connection<S: Io>(
        &self,
        cno: ConnectionNo,
        stream: S,
        remote: std::net::SocketAddr,
    ) -> Self::Conn;
}

mod shared;
pub use shared::*;
