use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;

use crate::util::mutex_poisoned;
use crate::{
    pcp::connection3::{Connection, ConnectionFactory, ConnectionHandle, ConnectionManager, ConnectionSpec},
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
    S: ConnectionSpec<Factory = Self>,
{
    manager: <S as ConnectionSpec>::Manager,
    marker: std::marker::PhantomData<S>,
}

impl<S> ConnectionFactory for SharedFactory<S>
where
    S: ConnectionSpec<Factory = Self>,
{
    type Spec = S;

    fn manager(&self) -> &<Self::Spec as ConnectionSpec>::Manager {
        &self.manager
    }

    fn create_accepted_connection(
        &self,
        cno: ConnectionNo,
        remote: std::net::SocketAddr,
        config: Option<<Self::Spec as ConnectionSpec>::ConnConfig>,
    ) -> <Self::Spec as ConnectionSpec>::Conn {
        let manager = self.manager.clone();
        let conn = S::Conn::new(cno, remote, config, manager);
        let handle = conn.handle();
        self.manager.insert(handle);
        conn
    }
}

impl<S> SharedFactory<S>
where
    S: ConnectionSpec<Factory = Self>,
{
    fn new(manager: <S as ConnectionSpec>::Manager) -> Self {
        Self {
            manager,
            marker: std::marker::PhantomData,
        }
    }
}
////////////////////////////////////////////////////////////////////////////////
// SharedConnection
//
pub fn shared_factory<S>() -> (SharedFactory<S>, SharedManager<S>)
where
    S: ConnectionSpec<Factory = SharedFactory<S>, Manager = SharedManager<S>>,
{
    let manager = SharedManager::<S>::new();
    let factory = SharedFactory::<S>::new(manager.clone());
    (factory, manager)
}
