use std::{
    future::Future,
    ops::Deref,
    sync::{Arc, Mutex},
};
use tracing::debug;

use crate::{
    pcp::{GnuId, ValidChannelInfo, ValidTrackInfo},
    repository::{typical_repository::TypicalRepository, Channel, Repository},
    util::mutex_poisoned,
};

/// 非公開の内部リポジトリ構造体
/// SharedRepository はこの構造体を Arc<Mutex<InnerRepository<C>> で保護します。
#[derive(Debug)]
struct InnerRepository<C> {
    repo: TypicalRepository<C>,
}

/// SharedRepository は マルチスレッドで共有されるリポジトリ実装です。
/// このリポジトリは Send(スレッド間の移動禁止) Sync(スレッド間の共有禁止)を実装しているため
/// 複数のスレッド間で安全に共有および移動されることを意図しています。
#[derive(Debug, Clone)]
pub struct SharedRepository<C>(Arc<Mutex<InnerRepository<C>>>);

impl<C> SharedRepository<C>
where
    C: Channel,
{
    const DELETE_WAIT_MINITES: u64 = 1; // TODO: 設定化, デフォルト値

    /// 新しい SharedRepository インスタンスを作成します。
    /// initialize_task 引数は、リポジトリの初期化時に呼び出される非同期タスクを指定します。
    pub async fn new<I, IFut>(initialize_task: I) -> Self
    where
        Self: Clone,
        I: FnOnce(SharedRepository<C>) -> IFut,
        IFut: Future<Output = ()> + Send + Sync,
    {
        let inner = InnerRepository {
            repo: TypicalRepository::<C>::new(),
        };
        let self_ = SharedRepository(Arc::new(Mutex::new(inner)));

        initialize_task(self_.clone()).await;
        self_
    }

    fn lock_impl<F, R>(&self, f: F) -> R
    where
        F: for<'a> FnOnce(&mut TypicalRepository<C>) -> R,
    {
        let mut guard = self.0.lock().unwrap_or_else(mutex_poisoned);
        f(&mut guard.repo)
    }
}

impl<C> Repository<C> for SharedRepository<C>
where
    C: Channel,
{
    fn get(&self, id: crate::pcp::GnuId) -> Option<C> {
        self.lock_impl(|repo| repo.get(id))
    }

    fn get_all(&self) -> Vec<C> {
        self.lock_impl(|repo| repo.get_all())
    }

    fn create(
        &self,
        id: GnuId,
        channel_info: Option<ValidChannelInfo>,
        track_info: Option<ValidTrackInfo>,
        config: Option<<C as Channel>::Config>,
    ) -> (C, bool) {
        self.lock_impl(|repo| repo.create(id, channel_info, track_info, config))
    }

    fn create_or_get(
        &self,
        id: crate::pcp::GnuId,
        channel_info: Option<ValidChannelInfo>,
        track_info: Option<ValidTrackInfo>,
        config: Option<C::Config>,
    ) -> impl Future<Output = C> + Send {
        let (mut ch, is_create) = { self.lock_impl(|repo| repo.create(id, channel_info, track_info, config)) };
        async move {
            if is_create {
                ch.after_create().await;
            }
            ch
        }
    }

    fn delete_channel(&self, id: crate::pcp::GnuId) -> bool {
        self.lock_impl(|repo| repo.delete_channel(id))
    }
    fn delete_all(&self) {
        self.lock_impl(|repo| repo.delete_all());
    }

    fn filter_map_collect<F, G, R>(&self, f: F, g: G) -> Vec<R>
    where
        F: FnMut(&GnuId, &C) -> bool,
        G: FnMut(&GnuId, &C) -> R,
    {
        self.lock_impl(|repo| repo.filter_map_collect(f, g))
    }
}

/*
/// 非同期で動作するチャンネルの削除タスクを管理する構造体
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
*/

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::{
        repository::{dummy_channel::DummyChannel, typical_repository::test_repository},
        test_helper::{assert_send, assert_sync},
    };

    fn test_have_send_trait() {
        type DummyInit = std::pin::Pin<Box<dyn Future<Output = ()> + Send>>;

        assert_send::<SharedRepository<DummyChannel>>();
        assert_sync::<SharedRepository<DummyChannel>>();
    }

    #[tokio::test]
    async fn test_shared_repository() {
        let mut repo: SharedRepository<DummyChannel> = SharedRepository::new(|s| async {}).await;
        test_repository(repo).await;
    }

    #[ignore = "not implemented yet"]
    #[tokio::test]
    async fn test_shared_repository_deleter_task() {
        let mut repo: SharedRepository<DummyChannel> = SharedRepository::new(|s| async move {
            s.get(GnuId::zero()).is_none();
            ()
        })
        .await;
    }
}
