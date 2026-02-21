use std::{
    collections::HashMap,
    fs::read,
    future::Future,
    sync::{Arc, Mutex, Weak},
    time::Duration,
};

use bytes::BytesMut;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::mpsc,
};

use crate::{
    pcp::{
        atom2::Atom2,
        connection2::{Connection, ConnectionFactory, ConnectionHandle, ConnectionManager, ConnectionStats, Io},
    },
    util::mutex_poisoned,
    ConnectionNo,
};

#[derive(Debug, thiserror::Error)]
enum SharedConnectionError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

////////////////////////////////////////////////////////////////////////////////
// utility for Connection
//
pub fn shared_connection_factory() -> (SharedConnectionFactory, Arc<SharedConnectionManager>) {
    let manager = Arc::new(SharedConnectionManager::new());
    let factory = SharedConnectionFactory::new(manager.clone());
    (factory, manager)
}

////////////////////////////////////////////////////////////////////////////////
// SharedConnectionManager
//
#[derive(Debug)]
pub struct SharedConnectionManager {
    conns: Arc<Mutex<HashMap<ConnectionNo, SharedConnectionHandle>>>,
}

impl SharedConnectionManager {
    pub fn new() -> Self {
        Self {
            conns: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl ConnectionManager<SharedConnectionHandle> for SharedConnectionManager {
    fn insert(&self, handle: SharedConnectionHandle) {
        let mut guard = self.conns.lock().unwrap_or_else(mutex_poisoned);
        guard.insert(handle.cno(), handle);
    }

    fn remove(&self, cno: &ConnectionNo) -> Option<SharedConnectionHandle> {
        let mut guard = self.conns.lock().unwrap_or_else(mutex_poisoned);
        guard.remove(cno)
    }

    fn report_metrics(&self) -> Vec<(ConnectionNo, Arc<crate::pcp::connection2::ConnectionStats>)> {
        self.conns.lock().unwrap_or_else(mutex_poisoned).iter().map(|(cno, handle)| (*cno, handle.stats())).collect()
    }
}

////////////////////////////////////////////////////////////////////////////////
//
//
pub enum Command {}

#[derive(Debug)]
pub struct SharedConnectionHandle {
    cno: ConnectionNo,
    tx: mpsc::Sender<Command>,
    stats: Arc<ConnectionStats>,
}
impl ConnectionHandle for SharedConnectionHandle {
    fn cno(&self) -> ConnectionNo {
        self.cno
    }

    fn stats(&self) -> Arc<ConnectionStats> {
        self.stats.clone()
    }
}

pub struct SharedConnection<S: Io> {
    cno: ConnectionNo,
    write_buf: BytesMut,
    read_buf: BytesMut,
    own_handle_tx: mpsc::Sender<Command>,
    own_handle_tr: mpsc::Receiver<Command>,
    stream: Option<S>,
    stats: Arc<ConnectionStats>,
    owner: Weak<SharedConnectionManager>,
}

#[async_trait::async_trait]
impl<S: Io> Connection for SharedConnection<S> {
    type Handle = SharedConnectionHandle;
    type Manager = SharedConnectionManager;

    fn cno(&self) -> ConnectionNo {
        self.cno
    }
    fn handle(&self) -> Self::Handle {
        SharedConnectionHandle {
            cno: self.cno,
            tx: self.own_handle_tx.clone(),
            stats: self.stats.clone(),
        }
    }
    fn stats(&self) -> Arc<ConnectionStats> {
        self.stats.clone()
    }
    fn owner(&self) -> &Weak<Self::Manager> {
        &self.owner
    }

    fn on_drop(&self) {
        // streamが存在する場合はrun()を実行していない状態なので、マネージャーから削除する必要がある
        if self.stream.is_some() {
            if let Some(manager) = self.owner().upgrade() {
                manager.remove(&self.cno());
            }
        }
    }

    async fn run(mut self) {
        let mut buff = BytesMut::with_capacity(4096);
        let (mut reader, mut writer) = tokio::io::split(self.stream.take().unwrap());

        // loop {
        //     tokio::select! {
        //         res = self.stream.read(&mut buf) => {
        //             match res {
        //                 Ok(0) => {
        //                     // EOF
        //                     break;
        //                 }
        //                 Ok(n) => {
        //                     println!("Read {} bytes: {:?}", n, &buf[..n]);
        //                     self.stats.add_bytes_received(n);
        //                 }
        //                 Err(e) => {
        //                     eprintln!("Error reading from stream: {}", e);
        //                     break;
        //                 }
        //             }
        //         }
        //         cmd = self.own_handle_tr.recv() => {
        //             match cmd {
        //                 Some(cmd) => {
        //                     println!("Received command: {:?}", cmd);
        //                 }
        //                 None => {
        //                     // Channel closed
        //                     break;
        //                 }
        //             }
        //         }
        //     }
        // }
    }
}

impl<S: Io> SharedConnection<S> {
    fn new(
        cno: ConnectionNo,
        stream: S,
        write_buf: BytesMut,
        read_buf: BytesMut,
        owner: &Arc<SharedConnectionManager>,
    ) -> Self {
        let (tx, tr) = mpsc::channel(1);
        let stats = Arc::new(ConnectionStats::new());

        Self {
            cno,
            stream: Some(stream),
            write_buf,
            read_buf,
            own_handle_tr: tr,
            own_handle_tx: tx,
            stats,
            owner: Arc::downgrade(owner),
        }
    }

    async fn send(&mut self, atom: Atom2) -> Result<(), SharedConnectionError> {
        if let Some(ref mut stream) = self.stream {
            // 送信処理
            atom.write_buf(&mut self.write_buf);
            stream.write_all(&self.write_buf).await?;
            self.write_buf.clear();
            Ok(())
        } else {
            return Err(SharedConnectionError::Io(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Connection is not active",
            )));
        }
    }

    async fn read(&mut self) -> Result<Atom2, SharedConnectionError> {
        if let Some(ref mut stream) = self.stream {
            // 読み取り処理
            let n = stream.read_buf(&mut self.read_buf).await?;
            if n == 0 {
                // EOF
                return Err(SharedConnectionError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Connection closed by peer",
                )));
            }

            // let atom = Atom2::from_buf(&self.read_buf[..n])?;
            // self.read_buf.advance(n);
            // Ok(atom)
            todo!()
        } else {
            return Err(SharedConnectionError::Io(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Connection is not active",
            )));
        }
    }
}

impl<S: Io> Drop for SharedConnection<S> {
    fn drop(&mut self) {
        self.on_drop();
    }
}

////////////////////////////////////////////////////////////////////////////////
//
//
#[derive(Debug)]
pub struct SharedConnectionFactory {
    manager: Arc<SharedConnectionManager>,
}

impl SharedConnectionFactory {
    const TIMEOUT_DURATION: std::time::Duration = Duration::from_secs(10);

    pub fn new(manager: Arc<SharedConnectionManager>) -> Self {
        Self {
            manager,
        }
    }
}

impl<S: Io> ConnectionFactory<S> for SharedConnectionFactory {
    type Conn = SharedConnection<S>;
    type Handle = SharedConnectionHandle;
    type Manager = SharedConnectionManager;

    fn manager(&self) -> &Self::Manager {
        &self.manager
    }

    fn create_accepted_connection(
        &self,
        cno: ConnectionNo,
        stream: S,
        remote: std::net::SocketAddr,
        write_buf: BytesMut,
        read_buf: BytesMut,
    ) -> Self::Conn {
        let conn = SharedConnection::new(cno, stream, write_buf, read_buf, &self.manager);
        let handle = conn.handle();
        self.manager.insert(handle);
        conn
    }

    fn create_outgoing_connection(&self, cno: ConnectionNo, stream: S, remote: std::net::SocketAddr) -> Self::Conn {
        let conn = SharedConnection::new(cno, stream, BytesMut::new(), BytesMut::new(), &self.manager);
        let handle = conn.handle();
        self.manager.insert(handle);
        conn
    }
}

#[cfg(test)]
mod t {
    use axum::serve;
    use bytes::BytesMut;
    use tokio::{
        io::{self, AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
    };

    use super::*;

    #[tokio::test]
    async fn test_shared_connection_manager() {
        let manager = Arc::new(SharedConnectionManager::new());
        let factory = SharedConnectionFactory::new(manager.clone());

        let cno1 = ConnectionNo::new();
        let cno2 = ConnectionNo::new();
        let (mut client, server) = io::duplex(1024);

        let server_addr = "127.0.0.1:1234".parse().unwrap();
        let client_addr = "127.0.0.1:4321".parse().unwrap();

        let conn_server =
            factory.create_accepted_connection(cno1, server, client_addr, BytesMut::new(), BytesMut::new());
        let conn_client = factory.create_outgoing_connection(cno2, client, server_addr);

        // conn_server.run().await;
        ()
    }

    #[tokio::test]
    async fn test_shared_connection_manager_stream() {
        let manager = Arc::new(SharedConnectionManager::new());
        let factory = SharedConnectionFactory::new(manager.clone());

        // TcpListener::bind(addr)
        let cno1 = ConnectionNo::new();
        let cno2 = ConnectionNo::new();
        let server_addr = "127.0.0.1";
        let listener_server = TcpListener::bind((server_addr, 0)).await.unwrap();
        let server_port = listener_server.local_addr().unwrap().port();

        let mut stream_local = TcpStream::connect(("127.0.0.1", server_port)).await.unwrap();
        stream_local.split();

        let (mut stream_server, remote) = listener_server.accept().await.unwrap();
        println!("remote: {:?}", &remote);
        println!("stream_local.local_addr(): {:?}", stream_local.local_addr());

        let _ = stream_local.write(b"abc").await;

        let mut buf = BytesMut::new();
        let n = stream_server.read_buf(&mut buf).await.unwrap();
        assert_eq!(n, 3);
        assert_eq!(&buf[..n], b"abc");

        // let conn_server = factory.create_accepted_connection(cno1, server, client_addr);
        // let conn_client = factory.create_outgoing_connection(cno2, client, server_addr);

        ()
    }
}
