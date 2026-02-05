use std::{
    future::Future,
    sync::{Arc, Mutex},
};
use tracing::debug;

use crate::{
    util::mutex_poisoned,
    {
        pcp::GnuId,
        repository::{inner_repository::InnerRepository, Channel, Repository},
    },
};

/// SharedRepository は マルチスレッドで共有されるリポジトリ実装です。
/// このリポジトリは Send(スレッド間の移動禁止) Sync(スレッド間の共有禁止)を実装しているため
/// 複数のスレッド間で安全に共有および移動されることを意図しています。
pub struct SharedRepository<C> {
    impl_: Arc<Mutex<InnerRepository<C>>>,
    deleter_task: DeleterTask,
}

impl<C: Channel> SharedRepository<C> {
    const DELETE_WAIT_MINITES: u64 = 1; // TODO: 設定化, デフォルト値

    pub fn new() -> Self {
        let impl_ = Arc::new(Mutex::new(InnerRepository::<C>::new()));
        let deleter_task = DeleterTask::new();
        deleter_task.start(impl_.clone(), Self::DELETE_WAIT_MINITES);

        SharedRepository {
            impl_,
            deleter_task,
        }
    }

    fn lock_impl(&self) -> std::sync::MutexGuard<'_, InnerRepository<C>> {
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

    fn filter_map_collect<F, G, R>(&self, f: F, g: G) -> Vec<R>
    where
        F: FnMut(&GnuId, &C) -> bool,
        G: FnMut(&GnuId, &C) -> R,
    {
        self.lock_impl().filter_map_collect(f, g)
    }
}

struct DeleterTask {
    handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl DeleterTask {
    fn new() -> Self {
        DeleterTask {
            handle: Arc::new(Mutex::new(None)),
        }
    }
    fn start<C: Channel>(&self, repo: Arc<Mutex<InnerRepository<C>>>, delete_wait_minutes: u64) -> bool {
        let mut guard = self.handle.lock().unwrap_or_else(mutex_poisoned);
        if let Some(_) = &*guard {
            return false;
        }
        *guard = Some(tokio::spawn(deleter_task(repo.clone(), delete_wait_minutes)));
        true
    }

    fn stop(&self) -> () {
        let mut guard = self.handle.lock().unwrap_or_else(mutex_poisoned);
        *guard = match &mut *guard {
            Some(h) => {
                h.abort();
                None
            }
            None => None,
        }
    }

    fn restart<C: Channel>(&self, repo: Arc<Mutex<InnerRepository<C>>>, delete_wait_minutes: u64) -> bool {
        self.stop();
        self.start(repo, delete_wait_minutes)
    }
}

async fn deleter_task<C: Channel>(repo: Arc<Mutex<InnerRepository<C>>>, delete_wait_minutes: u64) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
    loop {
        interval.tick().await;
        println!("SharedRepository deleter_task running...");
        {
            let mut repo_lock = repo.lock().unwrap_or_else(mutex_poisoned);
            // repo_lock.delete_all();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::{
        repository::{dummy_channel::DummyChannel, inner_repository::test_repository},
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

    #[ignore = "not implemented yet"]
    #[tokio::test]
    async fn test_shared_repository_deleter_task() {
        let mut repo: SharedRepository<DummyChannel> = SharedRepository::new();
    }
}
