use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::RwLock;

use tokio_util::sync::CancellationToken;

use crate::GnuId;
use crate::connection::ConnectionNo;
use crate::connection::{
    ConnectionFactory, ConnectionHandle, ConnectionManager, ConnectionSpec, HandshakeConnection, OutgoingConnection,
};
use crate::io::IoStream;
use crate::utils::sync::rwlock_read_poisoned;
use crate::utils::sync::rwlock_write_poisoned;

////////////////////////////////////////////////////////////////////////////////
/// SharedConnectionManager
///
#[derive(Debug)]
pub struct SharedManager<S>
where
    S: ConnectionSpec<Manager = Self>,
{
    conns: Arc<RwLock<HashMap<ConnectionNo, <S as ConnectionSpec>::Handle>>>,
}

impl<S> SharedManager<S>
where
    S: ConnectionSpec<Manager = Self>,
{
    pub fn register(&self, handle: <S as ConnectionSpec>::Handle) {
        self.conns.write().unwrap_or_else(rwlock_write_poisoned).insert(handle.cno(), handle.clone());
    }
    pub fn unregister(&self, handle: <S as ConnectionSpec>::Handle) {
        self.conns.write().unwrap_or_else(rwlock_write_poisoned).remove(&handle.cno());
    }
}

impl<S> Clone for SharedManager<S>
where
    S: ConnectionSpec<Manager = Self>,
{
    fn clone(&self) -> Self {
        Self {
            conns: Arc::clone(&self.conns),
        }
    }
}

impl<S> ConnectionManager for SharedManager<S>
where
    S: ConnectionSpec<Manager = Self>,
    Self: Clone,
{
    type Spec = S;

    fn find<P>(&self, predicate: P) -> Option<(ConnectionNo, <Self::Spec as ConnectionSpec>::Handle)>
    where
        P: Fn(&(&ConnectionNo, &<Self::Spec as ConnectionSpec>::Handle)) -> bool,
    {
        self.conns
            .read()
            .unwrap_or_else(rwlock_read_poisoned)
            .iter()
            .find(predicate)
            .map(|(cno, handle)| (cno.clone(), handle.clone()))
    }

    fn report_metrics(&self) -> Vec<(ConnectionNo, <Self::Spec as ConnectionSpec>::Stats)> {
        todo!()
    }

    fn get(&self, cno: &ConnectionNo) -> Option<<Self::Spec as ConnectionSpec>::Handle> {
        todo!()
    }

    fn delete(&self, cno: &ConnectionNo) -> Option<<Self::Spec as ConnectionSpec>::Handle> {
        todo!()
    }

    fn list(&self) -> Vec<(ConnectionNo, <Self::Spec as ConnectionSpec>::Handle)> {
        self.conns
            .read()
            .unwrap_or_else(rwlock_read_poisoned)
            .iter()
            .map(|(cno, handle)| (cno.clone(), handle.clone()))
            .collect()
    }
}

////////////////////////////////////////////////////////////////////////////////
/// SharedConnectionFactory
///
#[derive(Debug)]
pub struct SharedFactory<S>
where
    S: ConnectionSpec,
{
    self_session_id: GnuId,
    conns: Arc<RwLock<HashMap<ConnectionNo, <S as ConnectionSpec>::Handle>>>,
    manager: <S as ConnectionSpec>::Manager,
}

impl<S> ConnectionFactory for SharedFactory<S>
where
    S: ConnectionSpec,
{
    type Spec = S;
    type Handshake = <Self::Spec as ConnectionSpec>::Handshake;

    fn self_session_id(&self) -> peercast_gnuid::GnuId {
        self.self_session_id
    }

    fn manager(&self) -> &<Self::Spec as ConnectionSpec>::Manager {
        &self.manager
    }

    fn create_accepted_connection(
        &self,
        cno: ConnectionNo,
        stream: IoStream,
        remote: std::net::SocketAddr,
        config: Option<<Self::Spec as ConnectionSpec>::HandshakeConfig>,
    ) -> Self::Handshake {
        let manager = self.manager.clone();
        Self::Handshake::new(cno, stream, remote, config, manager)
    }

    fn create_outgoing<C>(&self, remote: SocketAddr, config: Option<<C as OutgoingConnection>::Config>) -> C
    where
        C: OutgoingConnection<Spec = Self::Spec>,
    {
        let manager = self.manager.clone();
        C::new(ConnectionNo::new(), remote, config, manager)
    }
}

////////////////////////////////////////////////////////////////////////////////
// connection_factory()
//
pub fn connection_factory<S>(self_session_id: GnuId) -> (SharedFactory<S>, SharedManager<S>)
where
    S: ConnectionSpec<Manager = SharedManager<S>>,
{
    let manager = SharedManager::<S> {
        conns: Arc::new(RwLock::new(HashMap::new())),
    };
    let factory = SharedFactory::<S> {
        self_session_id,
        conns: Arc::clone(&manager.conns),
        manager: manager.clone(),
    };

    (factory, manager)
}

#[cfg(test)]
mod tests {

    use crate::{connection::Connection, io::IoStream};

    use super::*;

    #[derive(Debug)]
    struct TestSpec {}
    impl ConnectionSpec for TestSpec {
        type Handle = TestHandle;
        type Manager = SharedManager<Self>;
        type Handshake = TestHandshake;
        type HandshakeConfig = ();
        type State = ();
        type Stats = ();
    }

    #[derive(Debug)]
    struct TestHandshake {
        cno: ConnectionNo,
        remote: SocketAddr,
        manager: SharedManager<TestSpec>,
        config: (),
    }
    #[async_trait::async_trait]
    impl HandshakeConnection for TestHandshake {
        type Spec = TestSpec;

        type HandshakeResult = Result<TestConnection, ()>;

        fn new(
            cno: ConnectionNo,
            stream: IoStream,
            remote: SocketAddr,
            config: Option<<Self::Spec as ConnectionSpec>::HandshakeConfig>,
            manager: <Self::Spec as ConnectionSpec>::Manager,
        ) -> Self {
            Self {
                cno,
                remote,
                manager,
                config: config.unwrap_or(()),
            }
        }

        fn manager(&self) -> &<Self::Spec as ConnectionSpec>::Manager {
            todo!()
        }

        fn cno(&self) -> ConnectionNo {
            self.cno
        }
        async fn handshake(self) -> Self::HandshakeResult {
            let new_conn = TestConnection {
                cno: self.cno,
                remote: self.remote,
                manager: self.manager,
                config: self.config,
            };
            new_conn.manager.register(TestHandle {
                cno: self.cno,
            });
            Ok(new_conn)
        }
    }

    #[derive(Debug)]
    struct TestConnection {
        cno: ConnectionNo,
        remote: SocketAddr,
        manager: SharedManager<TestSpec>,
        config: (),
    }

    impl Connection for TestConnection {
        type Spec = TestSpec;

        fn cno(&self) -> ConnectionNo {
            self.cno
        }

        fn remote(&self) -> SocketAddr {
            self.remote
        }

        fn handle(&self) -> <Self::Spec as ConnectionSpec>::Handle {
            TestHandle {
                cno: self.cno,
            }
        }

        fn run(self) -> impl Future<Output = Result<(), crate::connection::error::ConnectionError>> {
            async { todo!() }
        }
    }

    #[derive(Debug, Clone)]
    struct TestHandle {
        cno: ConnectionNo,
    }
    impl ConnectionHandle for TestHandle {
        type Spec = TestSpec;

        fn cno(&self) -> ConnectionNo {
            self.cno
        }

        fn task_name(&self) -> Arc<String> {
            todo!()
        }

        fn state(&self) -> <Self::Spec as ConnectionSpec>::State {
            todo!()
        }

        fn stats(&self) -> <Self::Spec as ConnectionSpec>::Stats {
            todo!()
        }

        fn shutdown(&self) -> () {
            todo!()
        }

        fn manager(&self) -> &<Self::Spec as ConnectionSpec>::Manager {
            todo!()
        }
    }

    #[tokio::test]
    async fn test_shared_factory() {
        let self_session_id = GnuId::new();
        let (factory, manager) = connection_factory::<TestSpec>(self_session_id);
        assert_eq!(factory.self_session_id(), self_session_id);

        let cno = ConnectionNo::new();
        let stream = tokio::io::duplex(64).0;
        let remote = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
        let config = Some(());

        let hshake = factory.create_accepted_connection(cno.clone(), stream.into(), remote.clone(), config);
        assert_eq!(hshake.cno(), cno);
        assert_eq!(manager.list().len(), 0); // ハンドシェイク完了前はコネクションはManagerに登録されない

        let conn = hshake.handshake().await.unwrap();
        assert_eq!(manager.list().len(), 1) // ハンドシェイク完了後はコネクションがManagerに登録される
    }
}
