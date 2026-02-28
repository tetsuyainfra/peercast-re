use std::{borrow::Cow, future::Future, net::SocketAddr};

use tokio_util::sync::CancellationToken;

use crate::{error::ConnectionError, io::Io, ConnectionNo};

/// Connection型の特性を定義するtrait
pub trait ConnectionSpec {
    // Handshakeを行う型 .handshake()を呼び出して次のEstablishedに遷移させる
    type Handshake: HandshakeConnection<Spec = Self>;
    type HandshakeConfig;

    //
    type State;
    type Stats;

    //
    type Handle: ConnectionHandle<Spec = Self> + Clone + std::fmt::Debug;
    type Manager: ConnectionManager<Spec = Self> + Clone + std::fmt::Debug;
    // type Factory: ConnectionFactory<Spec = Self>;
}

#[async_trait::async_trait]
pub trait HandshakeConnection: Sized {
    type Spec: ConnectionSpec<Handshake = Self>;
    // type Error;
    type HandshakeResult;

    fn new(
        cno: ConnectionNo,
        remote: SocketAddr,
        shutdown_token: Option<CancellationToken>,
        config: Option<<Self::Spec as ConnectionSpec>::HandshakeConfig>,
        manager: <Self::Spec as ConnectionSpec>::Manager,
    ) -> Self;

    fn manager(&self) -> &<Self::Spec as ConnectionSpec>::Manager;
    fn cno(&self) -> ConnectionNo;

    /// handshake()関数はSpecに定義されたEstablishdを実装した型を返す。
    /// その際、managerにEstablish::handle()を登録する。
    // async fn handshake(self) -> <Self::Spec as ConnectionSpec>::HandshakeResult;
    async fn handshake(self) -> Self::HandshakeResult;
}

////////////////////////////////////////////////////////////////////////////////
// ConnectionHandle
//
pub trait ConnectionHandle {
    type Spec: ConnectionSpec<Handle = Self>;

    fn cno(&self) -> ConnectionNo;

    fn task_name(&self) -> Cow<'static, str>;
    //  {
    //     Cow::Owned(format!("conn-{}", self.cno()))
    // }

    fn state(&self) -> <Self::Spec as ConnectionSpec>::State;
    fn stats(&self) -> <Self::Spec as ConnectionSpec>::Stats;

    fn shutdown(&self) -> ();
}

////////////////////////////////////////////////////////////////////////////////
// ConnectionManager
//
pub trait ConnectionManager {
    type Spec: ConnectionSpec<Manager = Self>;

    fn insert(&self, handle: <Self::Spec as ConnectionSpec>::Handle);
    fn remove(&self, cno: &ConnectionNo) -> Option<<Self::Spec as ConnectionSpec>::Handle>;

    // 実装予定
    // fn report_metrics(&self) -> Vec<(ConnectionNo, Arc<ConnectionStats>)>;
    fn report_metrics(&self) -> Vec<(ConnectionNo, <Self::Spec as ConnectionSpec>::Stats)>;
}

////////////////////////////////////////////////////////////////////////////////
// ConnectionFactory
//
pub trait ConnectionFactory {
    type Spec: ConnectionSpec;
    type Handshake: HandshakeConnection;

    fn manager(&self) -> &<Self::Spec as ConnectionSpec>::Manager;

    fn create_accepted_connection(
        &self,
        cno: ConnectionNo,
        remote: std::net::SocketAddr,
        shutdown_token: Option<CancellationToken>,
        config: Option<<Self::Spec as ConnectionSpec>::HandshakeConfig>,
    ) -> Self::Handshake;
}

////////////////////////////////////////////////////////////////////////////////
// Connection
// Connectionだけは独立して定義する。理由はマルチプロトコル対応のため
pub trait Connection: Sized {
    type Spec: ConnectionSpec;

    fn cno(&self) -> ConnectionNo;
    fn handle(&self) -> <Self::Spec as ConnectionSpec>::Handle;
    //
    fn run(self) -> impl Future<Output = Result<(), ConnectionError>>;
}

pub mod shared;

#[cfg(any(test, doc))]
mod example1;
#[cfg(any(test, doc))]
mod example2_multiple;
