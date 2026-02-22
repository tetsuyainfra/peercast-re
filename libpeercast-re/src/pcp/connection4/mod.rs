use std::net::SocketAddr;

use crate::ConnectionNo;
pub trait ConnectionSpec {
    // Handshakeを行う型 .handshake()を呼び出して次のEstablishedに遷移させる
    type Handshake: HandshakeConnection<Spec = Self>;
    type HandshakeConfig;

    // 通信の管理を実現する型 .run()を呼び出して使う
    type Established: EstablishedConnection<Spec = Self>;
    //
    type Stats: Send + Sync;
    //
    type Handle: ConnectionHandle<Spec = Self> + Clone;
    type Manager: ConnectionManager<Spec = Self> + Clone;
    // type Factory: ConnectionFactory<Spec = Self>;
}

#[async_trait::async_trait]
pub trait HandshakeConnection: Sized {
    type Spec: ConnectionSpec<Handshake = Self>;

    fn new(
        cno: ConnectionNo,
        remote: SocketAddr,
        config: Option<<Self::Spec as ConnectionSpec>::HandshakeConfig>,
        manager: <Self::Spec as ConnectionSpec>::Manager,
    ) -> Self;

    fn cno(&self) -> ConnectionNo;

    async fn handshake(self) -> <Self::Spec as ConnectionSpec>::Established;
}

#[async_trait::async_trait]
pub trait EstablishedConnection: Sized {
    type Spec: ConnectionSpec<Established = Self>;

    fn cno(&self) -> ConnectionNo;
    fn handle(&self) -> <Self::Spec as ConnectionSpec>::Handle;
    //
    async fn run(self);
}

pub trait ConnectionHandle {
    type Spec: ConnectionSpec<Handle = Self>;

    fn cno(&self) -> ConnectionNo;

    fn stats(&self) -> <Self::Spec as ConnectionSpec>::Stats;

    fn shutdown(&self) -> ();
}

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
        config: Option<<Self::Spec as ConnectionSpec>::HandshakeConfig>,
    ) -> Self::Handshake;
}

pub mod shared;

#[cfg(any(test, doc))]
mod example;
