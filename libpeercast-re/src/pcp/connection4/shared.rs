use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;

use crate::pcp::connection4::HandshakeConnection;
use crate::util::mutex_poisoned;
use crate::{
    pcp::connection4::{ConnectionFactory, ConnectionHandle, ConnectionManager, ConnectionSpec},
    ConnectionNo,
};
////////////////////////////////////////////////////////////////////////////////
/// SharedConnectionManager
///
pub struct SharedManager<S>
where
    S: ConnectionSpec<Manager = Self>,
{
    conns: Arc<Mutex<HashMap<ConnectionNo, <S as ConnectionSpec>::Handle>>>,
    marker: std::marker::PhantomData<S>,
}

impl<S> ConnectionManager for SharedManager<S>
where
    S: ConnectionSpec<Manager = Self>,
    Self: Clone,
{
    type Spec = S;

    fn insert(&self, handle: <Self::Spec as ConnectionSpec>::Handle) {
        self.conns.lock().unwrap_or_else(mutex_poisoned).insert(handle.cno(), handle);
    }

    fn remove(&self, cno: &ConnectionNo) -> Option<<Self::Spec as ConnectionSpec>::Handle> {
        self.conns.lock().unwrap_or_else(mutex_poisoned).remove(cno)
    }

    fn report_metrics(&self) -> Vec<(ConnectionNo, <Self::Spec as ConnectionSpec>::Stats)> {
        todo!()
    }
}

impl<S> SharedManager<S>
where
    S: ConnectionSpec<Manager = Self>,
{
    fn new() -> Self {
        Self {
            conns: Default::default(),
            marker: std::marker::PhantomData,
        }
    }
}

impl<S> Clone for SharedManager<S>
where
    S: ConnectionSpec<Manager = Self>,
{
    fn clone(&self) -> Self {
        Self {
            conns: Arc::clone(&self.conns),
            marker: std::marker::PhantomData,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
/// SharedConnectionFactory
///
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

//
////////////////////////////////////////////////////////////////////////////////
// SharedConnection
//
pub fn shared_factory<S>() -> (SharedFactory<S>, SharedManager<S>)
where
    S: ConnectionSpec<Manager = SharedManager<S>>,
{
    let manager = SharedManager::<S>::new();
    let factory = SharedFactory::<S>::new(manager.clone());
    (factory, manager)
}
