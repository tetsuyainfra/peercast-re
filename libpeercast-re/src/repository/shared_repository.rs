use std::{
    future::Future,
    sync::{Arc, Mutex},
};

use crate::{
    util::mutex_poisoned,
    {
        pcp::GnuId,
        repository::{impl_repository::_ImplRepository, Channel, Repository},
    },
};

/// SharedRepository は マルチスレッドで共有されるリポジトリ実装です。
/// このリポジトリは Send(スレッド間の移動禁止) Sync(スレッド間の共有禁止)を実装しているため
/// 複数のスレッド間で安全に共有および移動されることを意図しています。
pub struct SharedRepository<C> {
    impl_: Arc<Mutex<_ImplRepository<C>>>,
}

impl<C: Channel> SharedRepository<C> {
    pub fn new() -> Self {
        SharedRepository {
            impl_: Arc::new(Mutex::new(_ImplRepository::new())),
        }
    }

    fn lock_impl(&self) -> std::sync::MutexGuard<'_, _ImplRepository<C>> {
        self.impl_.lock().unwrap_or_else(mutex_poisoned)
    }
}

impl<C> Repository<C> for SharedRepository<C>
where
    C: Channel,
{
    fn get(&self, id: crate::pcp::GnuId) -> Option<C> {
        self.lock_impl().get(id)
    }

    fn get_all(&self) -> Vec<C> {
        self.lock_impl().get_all()
    }

    fn create(&mut self, id: GnuId, config: Option<<C as Channel>::Config>) -> (C, bool) {
        self.lock_impl().create(id, config)
    }

    fn create_or_get(&mut self, id: crate::pcp::GnuId, config: Option<C::Config>) -> impl Future<Output = C> + Send {
        let (mut ch, is_create) = { self.lock_impl().create(id, config) };
        async move {
            if is_create {
                ch.after_create().await;
            }
            ch
        }
    }

    fn delete_channel(&mut self, id: crate::pcp::GnuId) -> bool {
        self.lock_impl().delete_channel(id)
    }
    fn delete_all(&mut self) {
        self.lock_impl().delete_all();
    }

    fn map_collect<F, R>(&self, mut f: F) -> Vec<R>
    where
        F: FnMut(&GnuId, &C) -> R,
    {
        self.lock_impl().map_collect(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        repository::{dummy_channel::DummyChannel, impl_repository::test_repository},
        test_helper::{assert_send, assert_sync},
    };

    fn test_have_send_trait() {
        assert_send::<SharedRepository<DummyChannel>>();
        assert_sync::<SharedRepository<DummyChannel>>()
    }

    #[tokio::test]
    async fn test_shared_repository() {
        let mut repo: SharedRepository<DummyChannel> = SharedRepository::new();
        test_repository(repo).await;
    }
}
