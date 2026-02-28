/// Establishdを複数のプロトコルに対応させるサンプル
use std::{future::Future, net::SocketAddr};

use tokio::sync::{mpsc, watch};

use crate::{
    error::ConnectionError,
    pcp::connection5::{
        shared::{self, SharedFactory, SharedManager},
        Connection, ConnectionHandle, ConnectionSpec, HandshakeConnection,
    },
    ConnectionNo,
};

#[derive(Debug)]
pub struct MySpec();
impl ConnectionSpec for MySpec {
    type Handshake = MyHandshakeConnection;
    type HandshakeConfig = ();
    //
    type State = MyState;
    type Stats = MyStats;
    //
    type Handle = MyConnectionHandle;
    type Manager = SharedManager<Self>;
}

#[derive(Debug, Copy, Clone)]
pub enum MyState {
    Init,
    Running,
    ShuttingDown,
    Draining,
    Closed,
}

pub struct MyStats {}

enum Command {}

pub struct Handshake;
pub struct Established;

/// HandshakeConnectionの実装
/// 簡単化のため内部フィールドをArcにまとめていないが、Innerを定義してArc<Inner>にすると良い
pub struct MyHandshakeConnection {
    cno: ConnectionNo,
    remote: SocketAddr,
    config: Option<()>,
    shutdown_token: Option<tokio_util::sync::CancellationToken>,
    manager: <MySpec as ConnectionSpec>::Manager,
    //
    command_tx: mpsc::UnboundedSender<Command>,
    command_rx: mpsc::UnboundedReceiver<Command>,
    //
}

#[async_trait::async_trait]
impl HandshakeConnection for MyHandshakeConnection {
    type Spec = MySpec;
    type HandshakeResult = Result<MyEstablishedConnection, ()>;

    fn new(
        cno: ConnectionNo,
        remote: SocketAddr,
        shutdown_token: Option<tokio_util::sync::CancellationToken>,
        config: Option<<Self::Spec as ConnectionSpec>::HandshakeConfig>,
        manager: <Self::Spec as ConnectionSpec>::Manager,
    ) -> Self {
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        Self {
            cno,
            remote,
            config,
            shutdown_token,
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

    async fn handshake(self) -> Self::HandshakeResult {
        let (state_tx, state_rx) = watch::channel(MyState::Init);
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let established_connection = MyEstablishedConnection {
            cno: __self.cno,
            remote: __self.remote,
            config: __self.config,
            state_tx,
            state_rx,
            command_tx,
            command_rx,
            manager: __self.manager,
        };
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
    fn task_name(&self) -> std::borrow::Cow<'static, str> {
        todo!()
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

pub struct MyEstablishedConnection {
    cno: ConnectionNo,
    remote: SocketAddr,
    config: Option<()>,
    state_tx: watch::Sender<MyState>,
    state_rx: watch::Receiver<MyState>,
    command_tx: mpsc::UnboundedSender<Command>,
    command_rx: mpsc::UnboundedReceiver<Command>,
    manager: <MySpec as ConnectionSpec>::Manager,
}

impl Connection for MyEstablishedConnection {
    type Spec = MySpec;

    fn cno(&self) -> ConnectionNo {
        self.cno
    }

    fn handle(&self) -> <Self::Spec as ConnectionSpec>::Handle {
        MyConnectionHandle {
            cno: self.cno,
            remote: self.remote,
            config: self.config,
            command_tx: self.command_tx.clone(),
        }
    }

    fn run(self) -> impl Future<Output = Result<(), ConnectionError>> {
        async { Ok(()) }
    }
}

#[cfg(test)]
mod t {
    use std::net::SocketAddr;

    use crate::pcp::connection5::{ConnectionFactory, ConnectionManager};
    use crate::ConnectionNo;

    use super::*;

    #[tokio::test]
    async fn test() {
        let (factory, manager) = shared::connection_factory::<MySpec>();

        let cno = ConnectionNo::new();
        let remote: SocketAddr = "127.0.0.1:7144".parse().unwrap();
        let accept = factory.create_accepted_connection(cno, remote, None, Some(()));

        let r_connection = accept.handshake().await;
        assert!(r_connection.is_ok());
        let connection = r_connection.unwrap();

        assert_eq!(connection.cno(), cno);
        manager.insert(connection.handle());
    }
}
