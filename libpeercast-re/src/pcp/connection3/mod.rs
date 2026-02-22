use std::net::SocketAddr;

use crate::ConnectionNo;
pub trait ConnectionSpec {
    type Conn: Connection<Spec = Self>;
    type ConnConfig;
    type Stats: Send + Sync;
    // type ConnectionStats: Send + Sync + 'static;
    type Handle: ConnectionHandle<Spec = Self> + Clone;
    type Manager: ConnectionManager<Spec = Self> + Clone;
    type Factory: ConnectionFactory<Spec = Self>;
}

#[async_trait::async_trait]
pub trait Connection: Sized {
    type Spec: ConnectionSpec<Conn = Self>;

    fn new(
        cno: ConnectionNo,
        remote: SocketAddr,
        config: Option<<Self::Spec as ConnectionSpec>::ConnConfig>,
        manager: <Self::Spec as ConnectionSpec>::Manager,
    ) -> Self;

    fn cno(&self) -> ConnectionNo;
    fn handle(&self) -> <Self::Spec as ConnectionSpec>::Handle;

    //
    fn on_drop(&self);
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

pub trait ConnectionFactory {
    type Spec: ConnectionSpec<Factory = Self>;

    fn manager(&self) -> &<Self::Spec as ConnectionSpec>::Manager;

    fn create_accepted_connection(
        &self,
        cno: ConnectionNo,
        remote: std::net::SocketAddr,
        config: Option<<Self::Spec as ConnectionSpec>::ConnConfig>,
    ) -> <Self::Spec as ConnectionSpec>::Conn;
}

pub mod shared;

#[cfg(any(test, doc))]
mod example;

#[cfg(any(test, doc))]
mod example2;
