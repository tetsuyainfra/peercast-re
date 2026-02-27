use std::net::SocketAddr;

use tokio::sync::mpsc;

use crate::{
    pcp::connection4::{
        shared::{self, SharedFactory, SharedManager},
        ConnectionHandle, ConnectionManager, ConnectionSpec, EstablishedConnection, HandshakeConnection,
    },
    ConnectionNo,
};

#[derive(Debug)]
pub struct MySpec();
impl ConnectionSpec for MySpec {
    type Handshake = MyHandshakeConnection;
    type HandshakeConfig = ();
    //
    type Established = MyEstablishedConnection;
    type State = MyState;
    type Stats = MyStats;
    type Handle = MyConnectionHandle;
    type Manager = SharedManager<Self>;
}

pub struct MyState {}
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
    type Error = ();

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
    fn manager(&self) -> &<Self::Spec as ConnectionSpec>::Manager {
        &self.manager
    }

    async fn handshake(self) -> Result<<Self::Spec as ConnectionSpec>::Established, Self::Error> {
        let Self {
            cno,
            remote,
            config,
            manager,
            command_tx,
            command_rx,
        } = self;
        // something do handshake....

        // create EstablishedConnection
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let established_connection = MyEstablishedConnection::A(MyInner::<ProtA> {
            cno,
            remote,
            config,
            manager: manager.clone(),
            command_tx,
            command_rx,
            protocol: ProtA {},
        });

        let handle = established_connection.handle();
        manager.insert(handle);

        Ok(established_connection)
    }
}

#[derive(Debug, Clone)]
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

    fn state(&self) -> <Self::Spec as ConnectionSpec>::State {
        todo!()
    }
    fn stats(&self) -> <Self::Spec as ConnectionSpec>::Stats {
        // self.inner.stats.snapshot()
        todo!()
    }

    fn shutdown(&self) -> () {
        todo!()
    }
}

pub enum MyEstablishedConnection {
    A(MyInner<ProtA>),
    B(MyInner<ProtB>),
}

impl EstablishedConnection for MyEstablishedConnection {
    type Spec = MySpec;

    fn cno(&self) -> ConnectionNo {
        match self {
            MyEstablishedConnection::A(a) => a.cno,
            MyEstablishedConnection::B(b) => b.cno,
        }
    }
    fn handle(&self) -> <Self::Spec as ConnectionSpec>::Handle {
        match self {
            MyEstablishedConnection::A(a) => a.handle(),
            MyEstablishedConnection::B(b) => b.handle(),
        }
    }
    //
    fn run(self) -> impl std::future::Future<Output = Result<(), crate::error::ConnectionError>> {
        async move { Ok(()) }
    }
}

pub struct ProtA {}
pub struct ProtB {}

pub struct MyInner<S> {
    cno: ConnectionNo,
    remote: SocketAddr,
    config: Option<()>,
    manager: <MySpec as ConnectionSpec>::Manager,
    command_tx: mpsc::UnboundedSender<Command>,
    command_rx: mpsc::UnboundedReceiver<Command>,

    protocol: S,
}
impl<S> MyInner<S> {
    fn handle(&self) -> MyConnectionHandle {
        MyConnectionHandle {
            cno: self.cno,
            remote: self.remote,
            config: self.config,
            command_tx: self.command_tx.clone(),
        }
    }
}
impl MyInner<ProtA> {}

impl MyInner<ProtB> {}

#[cfg(test)]
mod t {
    use std::net::SocketAddr;

    use crate::pcp::connection4::ConnectionFactory;
    use crate::ConnectionNo;

    use super::*;

    #[tokio::test]
    async fn test() {
        let (factory, manager) = shared::connection_factory::<MySpec>();

        let cno = ConnectionNo::new();
        let remote: SocketAddr = "127.0.0.1:7144".parse().unwrap();
        let accept = factory.create_accepted_connection(cno, remote, Some(()));

        let connection = accept.handshake().await.unwrap();
        let _ = connection.run().await;
    }
}
