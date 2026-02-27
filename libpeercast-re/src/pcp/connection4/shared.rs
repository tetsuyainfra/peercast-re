use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;

use tokio_util::sync::CancellationToken;

use crate::pcp::connection4::HandshakeConnection;
use crate::util::mutex_poisoned;
use crate::{
    pcp::connection4::{ConnectionFactory, ConnectionHandle, ConnectionManager, ConnectionSpec},
    ConnectionNo,
};
////////////////////////////////////////////////////////////////////////////////
/// SharedConnectionManager
///
#[derive(Debug)]
pub struct SharedManager<S>
where
    S: ConnectionSpec<Manager = Self>,
{
    inner: Arc<Mutex<SMInner<<S as ConnectionSpec>::Handle>>>,
    // marker: std::marker::PhantomData<S>,
}

impl<S> ConnectionManager for SharedManager<S>
where
    S: ConnectionSpec<Manager = Self>,
    Self: Clone,
{
    type Spec = S;

    fn insert(&self, handle: <Self::Spec as ConnectionSpec>::Handle) {
        self.inner.lock().unwrap_or_else(mutex_poisoned).insert(handle.cno(), handle);
    }

    fn remove(&self, cno: &ConnectionNo) -> Option<<Self::Spec as ConnectionSpec>::Handle> {
        self.inner.lock().unwrap_or_else(mutex_poisoned).remove(cno)
    }

    fn report_metrics(&self) -> Vec<(ConnectionNo, <Self::Spec as ConnectionSpec>::Stats)> {
        todo!()
    }
}

impl<S> SharedManager<S>
where
    S: ConnectionSpec<Manager = Self>,
{
    fn new(shutdown_token: Option<CancellationToken>) -> Self {
        let inner = SMInner::new(shutdown_token);
        Self {
            inner: Arc::new(Mutex::new(inner)),
        }
    }
}

impl<S> Clone for SharedManager<S>
where
    S: ConnectionSpec<Manager = Self>,
{
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

#[derive(Debug)]
struct SMInner<H> {
    conns: HashMap<ConnectionNo, H>,
    shutdown_token: CancellationToken,
}
impl<H> SMInner<H> {
    fn new(shutdown_token: Option<CancellationToken>) -> Self {
        Self {
            conns: Default::default(),
            shutdown_token: shutdown_token.unwrap_or_else(|| CancellationToken::new()), // marker: std::marker::PhantomData,
        }
    }

    fn insert(&mut self, cno: ConnectionNo, handle: H) {
        self.conns.insert(cno, handle);
    }

    fn remove(&mut self, cno: &ConnectionNo) -> Option<H> {
        self.conns.remove(cno)
    }

    fn report_metrics<St>(&self) -> Vec<(ConnectionNo, St)> {
        todo!()
    }
}

impl<H> Default for SMInner<H> {
    fn default() -> Self {
        Self {
            conns: Default::default(),
            shutdown_token: CancellationToken::new(),
        }
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
    manager: <S as ConnectionSpec>::Manager,
    marker: std::marker::PhantomData<S>,
}

impl<S> ConnectionFactory for SharedFactory<S>
where
    S: ConnectionSpec,
{
    type Spec = S;
    type Handshake = <Self::Spec as ConnectionSpec>::Handshake;

    fn manager(&self) -> &<Self::Spec as ConnectionSpec>::Manager {
        &self.manager
    }

    fn create_accepted_connection(
        &self,
        cno: ConnectionNo,
        remote: std::net::SocketAddr,
        config: Option<<Self::Spec as ConnectionSpec>::HandshakeConfig>,
    ) -> Self::Handshake {
        let manager = self.manager.clone();
        Self::Handshake::new(cno, remote, config, manager)
    }
}

impl<S> SharedFactory<S>
where
    S: ConnectionSpec,
{
    fn new(manager: <S as ConnectionSpec>::Manager) -> Self {
        Self {
            manager,
            marker: std::marker::PhantomData,
        }
    }
}
////////////////////////////////////////////////////////////////////////////////
// HandshakeConnection
//

////////////////////////////////////////////////////////////////////////////////
// connection_factory()
//
pub fn connection_factory<S>(shutdown_token: Option<CancellationToken>) -> (SharedFactory<S>, SharedManager<S>)
where
    S: ConnectionSpec<Manager = SharedManager<S>>,
{
    let manager = SharedManager::<S>::new(shutdown_token);
    let factory = SharedFactory::<S>::new(manager.clone());
    (factory, manager)
}
