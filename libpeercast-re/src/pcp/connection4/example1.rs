/// Establishdを複数のプロトコルに対応させるサンプル
use std::net::SocketAddr;

use tokio::sync::{mpsc, watch};

use crate::{
    pcp::connection4::{
        shared::{self, SharedFactory, SharedManager},
        ConnectionHandle, ConnectionSpec, EstablishedConnection, HandshakeConnection,
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
    manager: <MySpec as ConnectionSpec>::Manager,
    //
    command_tx: mpsc::UnboundedSender<Command>,
    command_rx: mpsc::UnboundedReceiver<Command>,
    //
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
        let (state_tx, state_rx) = watch::channel(MyState::Init);
        let established_connection = MyEstablishedConnection {
            cno: self.cno,
            remote: self.remote,
            config: self.config,
            state_tx,
            state_rx,
            manager: self.manager,
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
    async fn run(mut self) {
        loop {
            let now_state = self.state_tx.borrow().clone();
            let new_state = match now_state {
                MyState::Init => self.on_init().await,
                // MyState::Running => self.on_running().await,
                // MyState::ShuttingDown => self.on_shutting_down().await,
                // MyState::Draining => self.on_draining().await,
                _ => todo!(),
                MyState::Closed => break,
            };
            self.state_tx.send(new_state).unwrap();
        }
    }
}

impl MyEstablishedConnection {
    async fn on_init(&mut self) -> MyState {
        // handshake / auth / setup
        MyState::Closed
    }

    // async fn on_running(&mut self) -> ConnState {
    //     tokio::select! {
    //         res = self.read_one() => {
    //             if res.is_err() {
    //                 return ConnState::ShuttingDown;
    //             }
    //         }
    //         _ = self.shutdown.wait() => {
    //             return ConnState::ShuttingDown;
    //         }
    //     }
    //     ConnState::Running
    // }

    // async fn on_shutting_down(&mut self) -> ConnState {
    //     // self.disable_read();
    //     // ConnState::Draining
    // }

    // async fn on_draining(&mut self) -> ConnState {
    //     // if self.write_buffer_empty() {
    //     //     self.flush().await;
    //     //     ConnState::Closed
    //     // } else {
    //     //     self.flush_some().await;
    //     //     ConnState::Draining
    //     // }
    // }
}

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

        let r_connection = accept.handshake().await;
        assert!(r_connection.is_ok());
        let connection = r_connection.unwrap();

        assert_eq!(connection.cno(), cno);
    }
}
