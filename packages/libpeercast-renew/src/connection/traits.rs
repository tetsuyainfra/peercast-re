use std::{future::Future, net::SocketAddr, sync::Arc};

use peercast_gnuid::GnuId;
use tokio_util::sync::CancellationToken;

use crate::{
    connection::{ConnectionNo, error::ConnectionError},
    io::IoStream,
};

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
        stream: IoStream,
        remote: SocketAddr,
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
    fn manager(&self) -> &<Self::Spec as ConnectionSpec>::Manager;

    fn task_name(&self) -> Arc<String>;
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

    fn get(&self, cno: &ConnectionNo) -> Option<<Self::Spec as ConnectionSpec>::Handle>;
    fn delete(&self, cno: &ConnectionNo) -> Option<<Self::Spec as ConnectionSpec>::Handle>;
    fn list(&self) -> Vec<(ConnectionNo, <Self::Spec as ConnectionSpec>::Handle)>;
    fn find<P>(&self, predicate: P) -> Option<(ConnectionNo, <Self::Spec as ConnectionSpec>::Handle)>
    where
        P: Fn(&(&ConnectionNo, &<Self::Spec as ConnectionSpec>::Handle)) -> bool;

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

    fn self_session_id(&self) -> GnuId;

    fn manager(&self) -> &<Self::Spec as ConnectionSpec>::Manager;

    fn create_accepted_connection(
        &self,
        cno: ConnectionNo,
        stream: IoStream,
        remote: std::net::SocketAddr,
        config: Option<<Self::Spec as ConnectionSpec>::HandshakeConfig>,
    ) -> Self::Handshake;

    fn create_outgoing<C>(&self, remote: std::net::SocketAddr, config: Option<<C as OutgoingConnection>::Config>) -> C
    where
        C: OutgoingConnection<Spec = Self::Spec>;

    // fn ping_to(
    //     &self,
    //     remote: std::net::SocketAddr,
    //     remote_session_id: Option<GnuId>,
    // ) -> impl Future<Output = Result<(), PingError>> {
    //     async {
    //         //
    //         let atoms = PingBuilder::new(self.self_session_id()).port(None).port_check(None).build();

    //         // let mut framed = Framed::new(stream, AtomCodec::new());
    //         // for a in atoms {
    //         //     dbg!("send:  {:?}", &a);
    //         //     framed.send(a).await?;
    //         // }

    //         // let mut atom = framed.next().await;
    //         // let oleh_candidate = match atom {
    //         //     None => todo!(),
    //         //     Some(Err(e)) => todo!(),
    //         //     Some(Ok(a)) => a,
    //         // };

    //         // let oleh = OlehInfo::try_from(&oleh_candidate).map_err(|_| HandshakeError::Failed)?;

    //         // let parts = PartsWrapFramed {
    //         //     cno,
    //         //     remote,
    //         //     framed,
    //         // };
    //         // let ret = HandshakeResult {
    //         //     parts,
    //         // };

    //         // Ok((oleh, ret))

    //         Ok(())
    //     }
    // }
}

////////////////////////////////////////////////////////////////////////////////
// Connection
// Connectionだけは独立して定義する。理由はマルチプロトコル対応のため
pub trait Connection: Sized {
    type Spec: ConnectionSpec;

    fn cno(&self) -> ConnectionNo;
    fn remote(&self) -> SocketAddr;
    fn handle(&self) -> <Self::Spec as ConnectionSpec>::Handle;
    //
    fn run(self) -> impl Future<Output = Result<(), ConnectionError>>;
}

/// OutgoingConnection
pub trait OutgoingConnection: Connection {
    type Config;
    type Output;
    fn new(
        cno: ConnectionNo,
        remote: std::net::SocketAddr,
        config: Option<Self::Config>,
        manager: <Self::Spec as ConnectionSpec>::Manager,
    ) -> Self;

    fn connect(self) -> impl Future<Output = Self::Output>;
}
