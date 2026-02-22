use std::net::SocketAddr;

use tokio::sync::mpsc;

use crate::{
    pcp::connection3::{
        shared::{self, SharedFactory, SharedManager},
        Connection, ConnectionHandle, ConnectionSpec,
    },
    ConnectionNo,
};

pub struct MySpec();
impl ConnectionSpec for MySpec {
    type Conn = MyConnection<Established>;
    type ConnConfig = ();
    type Stats = MyStats;
    type Handle = MyConnectionHandle;
    type Manager = SharedManager<Self>;
    type Factory = SharedFactory<Self>;
}

pub struct MyStats {}

enum Command {}

pub struct Handshake;
pub struct Established;

pub struct MyConnection<S> {
    cno: ConnectionNo,
    remote: SocketAddr,
    config: Option<()>,
    manager: <MySpec as ConnectionSpec>::Manager,
    //
    command_tx: mpsc::UnboundedSender<Command>,
    command_rx: mpsc::UnboundedReceiver<Command>,

    marker: std::marker::PhantomData<S>,
}

impl MyConnection<Handshake> {
    async fn handshake(mut self) -> MyConnection<Established> {
        // handshake処理
        MyConnection::<Established> {
            cno: self.cno,
            remote: self.remote,
            config: self.config,
            manager: self.manager,
            command_tx: self.command_tx,
            command_rx: self.command_rx,
            //
            marker: std::marker::PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl Connection for MyConnection<Established> {
    type Spec = MySpec;

    fn new(
        cno: ConnectionNo,
        remote: SocketAddr,
        config: Option<<Self::Spec as ConnectionSpec>::ConnConfig>,
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
            marker: std::marker::PhantomData,
        }
    }

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

    fn on_drop(&self) -> () {
        todo!()
    }

    async fn run(self) {
        todo!()
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

#[cfg(test)]
mod t {
    use std::net::SocketAddr;

    use crate::pcp::connection3::ConnectionFactory;
    use crate::ConnectionNo;

    use super::*;

    #[test]
    fn test() {
        let (factory, manager) = shared::shared_factory::<MySpec>();

        let cno = ConnectionNo::new();
        let remote: SocketAddr = "127.0.0.1:7144".parse().unwrap();
        let connection = factory.create_accepted_connection(cno, remote, Some(()));

        assert_eq!(connection.cno(), cno);
    }
}
