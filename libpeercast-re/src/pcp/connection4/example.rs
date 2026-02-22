use std::net::SocketAddr;

use tokio::sync::mpsc;

use crate::{
    pcp::connection4::{
        shared::{self, SharedFactory, SharedManager},
        ConnectionHandle, ConnectionSpec, EstablishedConnection, HandshakeConnection,
    },
    ConnectionNo,
};

pub struct MySpec();
impl ConnectionSpec for MySpec {
    type Handshake = MyHandshakeConnection;
    type HandshakeConfig = ();
    //
    type Established = MyEstablishedConnection;
    type Stats = MyStats;
    type Handle = MyConnectionHandle;
    type Manager = SharedManager<Self>;
}

pub struct MyStats {}

enum Command {}

pub struct Handshake;
pub struct Established;

pub struct MyHandshakeConnection {
    cno: ConnectionNo,
    remote: SocketAddr,
    config: Option<()>,
    manager: <MySpec as ConnectionSpec>::Manager,
    //
    command_tx: mpsc::UnboundedSender<Command>,
    command_rx: mpsc::UnboundedReceiver<Command>,
    //
    // marker: std::marker::PhantomData<S>,
}

#[async_trait::async_trait]
impl HandshakeConnection for MyHandshakeConnection {
    type Spec = MySpec;

    fn new(
        cno: ConnectionNo,
        remote: SocketAddr,
        config: Option<<Self::Spec as ConnectionSpec>::HandshakeConfig>,
        manager: <Self::Spec as ConnectionSpec>::Manager,
    ) -> Self {
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        Self {
            cno,
            remote,
            config,
            manager,
            command_tx,
            command_rx,
            // marker: std::marker::PhantomData,
        }
    }

    fn cno(&self) -> ConnectionNo {
        self.cno
    }

    async fn handshake(self) -> <Self::Spec as ConnectionSpec>::Established {
        MyEstablishedConnection {
            cno: self.cno,
            remote: self.remote,
            config: self.config,
            manager: self.manager,
        }
    }
}

#[derive(Clone)]
pub struct MyConnectionHandle {
    cno: ConnectionNo,
    remote: SocketAddr,
    config: Option<()>,
    //
    command_tx: mpsc::UnboundedSender<Command>,
}

impl ConnectionHandle for MyConnectionHandle {
    type Spec = MySpec;

    fn cno(&self) -> crate::ConnectionNo {
        self.cno
    }

    fn stats(&self) -> <Self::Spec as ConnectionSpec>::Stats {
        // self.inner.stats.snapshot()
        todo!()
    }

    fn shutdown(&self) -> () {
        todo!()
    }
}

pub struct MyEstablishedConnection {
    cno: ConnectionNo,
    remote: SocketAddr,
    config: Option<()>,
    manager: <MySpec as ConnectionSpec>::Manager,
}

#[async_trait::async_trait]
impl EstablishedConnection for MyEstablishedConnection {
    type Spec = MySpec;

    fn cno(&self) -> ConnectionNo {
        self.cno
    }
    fn handle(&self) -> <Self::Spec as ConnectionSpec>::Handle {
        todo!()
    }
    //
    async fn run(self) {}
}

#[cfg(test)]
mod t {
    use std::net::SocketAddr;

    use crate::pcp::connection4::ConnectionFactory;
    use crate::ConnectionNo;

    use super::*;

    #[tokio::test]
    async fn test() {
        let (factory, manager) = shared::shared_factory::<MySpec>();

        let cno = ConnectionNo::new();
        let remote: SocketAddr = "127.0.0.1:7144".parse().unwrap();
        let accept = factory.create_accepted_connection(cno, remote, Some(()));

        let connection = accept.handshake().await;

        assert_eq!(connection.cno(), cno);
    }
}
