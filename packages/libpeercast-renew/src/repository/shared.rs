use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use peercast_gnuid::GnuId;

use crate::model::{ValidChannelInfo, ValidTrackInfo};
use crate::repository::traits::*;
use crate::runtime::spawner::{Spawner, TokioSpawner};
use crate::utils::sync::{rwlock_read_poisoned, rwlock_write_poisoned};

////////////////////////////////////////////////////////////////////////////////
/// SharedChannelRepository
///
#[derive(Debug)]
pub struct SharedChannelRepository<S>
where
    S: RepositorySpec<Repository = Self>,
{
    channels: Arc<RwLock<HashMap<GnuId, S::Channel>>>,
}

impl<S> Clone for SharedChannelRepository<S>
where
    S: RepositorySpec<Repository = Self>,
{
    fn clone(&self) -> Self {
        Self {
            channels: self.channels.clone(),
        }
    }
}

impl<S> ChannelRepository for SharedChannelRepository<S>
where
    S: RepositorySpec<Repository = Self>,
{
    type Spec = S;

    fn get_channel(&self, id: &GnuId) -> Option<<Self::Spec as RepositorySpec>::Handle> {
        self.channels.read().unwrap_or_else(rwlock_read_poisoned).get(id).map(|c| c.handle())
    }

    fn delete_channel(&self, id: &GnuId) -> Option<<Self::Spec as RepositorySpec>::Handle> {
        self.channels.write().unwrap_or_else(rwlock_write_poisoned).remove(id).map(|c| c.handle())
    }

    fn list_channels(&self) -> Vec<<Self::Spec as RepositorySpec>::Handle> {
        self.channels.read().unwrap_or_else(rwlock_read_poisoned).values().map(|c| c.handle()).collect()
    }
}

////////////////////////////////////////////////////////////////////////////////
/// SharedChannelFactory
///
#[derive(Debug)]
pub struct SharedChannelFactory<S>
where
    S: RepositorySpec,
{
    spawner: S::Spawner,
    self_session_id: Arc<GnuId>,
    channels: Arc<RwLock<HashMap<GnuId, S::Channel>>>,
    repository: S::Repository,
}

impl<S> ChannelFactory for SharedChannelFactory<S>
where
    S: RepositorySpec,
    S::Spawner: Spawner,
{
    type Spec = S;

    fn self_session_id(&self) -> &GnuId {
        &self.self_session_id
    }

    fn create_or_get(
        &self,
        id: GnuId,
        valid_info: ValidChannelInfo,
        valid_track: ValidTrackInfo,
        config: <Self::Spec as RepositorySpec>::Config,
    ) -> <Self::Spec as RepositorySpec>::Handle {
        let mut channels = self.channels.write().unwrap_or_else(rwlock_write_poisoned);
        channels
            .entry(id)
            .or_insert_with_key(|id| {
                //
                S::Channel::new(&self.spawner, *id, valid_info, valid_track, config, self.repository.clone())
            })
            .handle()
    }
}

////////////////////////////////////////////////////////////////////////////////
/// SharedChannelRepository/SharedChannelFactoryを生成するための関数
/// Spawner = TokioSpanerの場合専用
pub fn channel_factory<S>(self_session_id: Arc<GnuId>) -> (SharedChannelFactory<S>, SharedChannelRepository<S>)
where
    S: RepositorySpec<Repository = SharedChannelRepository<S>, Spawner = TokioSpawner>,
{
    let repository = SharedChannelRepository::<S> {
        channels: Arc::new(RwLock::new(HashMap::new())),
    };

    let factory = SharedChannelFactory::<S> {
        spawner: TokioSpawner,
        self_session_id,
        channels: Arc::clone(&repository.channels),
        repository: repository.clone(),
    };

    (factory, repository)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestSpec {}
    impl RepositorySpec for TestSpec {
        type Channel = TestChannel;
        type Config = ();
        type Handle = TestChannelHandle;
        type Spawner = TokioSpawner;

        type State = ();
        type Stats = ();

        type Repository = SharedChannelRepository<Self>;
    }

    #[derive(Debug)]
    struct TestChannel {
        id: Arc<GnuId>,
        manager: SharedChannelRepository<TestSpec>,
        config: (),
        spawned: Arc<tokio::task::JoinHandle<()>>,
    }
    impl TestChannel {}
    impl Channel for TestChannel {
        type Spec = TestSpec;

        fn new(
            spawner: &<Self::Spec as RepositorySpec>::Spawner,
            id: GnuId,
            valid_info: ValidChannelInfo,
            valid_track: ValidTrackInfo,
            config: <Self::Spec as RepositorySpec>::Config,
            manager: <Self::Spec as RepositorySpec>::Repository,
        ) -> Self {
            let spawned = spawner.spawn(async {
                // ここで非同期に初期化処理を行うことができる
            });

            Self {
                id: Arc::new(id),
                manager,
                config: (),
                spawned: Arc::new(spawned),
            }
        }

        fn id(&self) -> &GnuId {
            &self.id
        }

        fn handle(&self) -> <Self::Spec as RepositorySpec>::Handle {
            TestChannelHandle {
                id: Arc::clone(&self.id),
                config: (),
                spawned: Arc::clone(&self.spawned),
            }
        }
    }

    #[derive(Debug, Clone)]
    struct TestChannelHandle {
        id: Arc<GnuId>,
        config: (),
        spawned: Arc<tokio::task::JoinHandle<()>>,
    }
    impl TestChannelHandle {
        fn check_spawned(&self) -> bool {
            self.spawned.is_finished()
        }
    }

    impl ChannelHandle for TestChannelHandle {
        type Spec = TestSpec;

        fn id(&self) -> &GnuId {
            &self.id
        }

        fn config(&self) -> &<Self::Spec as RepositorySpec>::Config {
            &self.config
        }

        fn state(&self) -> &<Self::Spec as RepositorySpec>::State {
            todo!()
        }

        fn stats(&self) -> &<Self::Spec as RepositorySpec>::Stats {
            todo!()
        }

        fn channel_meta(&self) -> crate::model::ChannelMeta {
            todo!()
        }
    }

    #[tokio::test]
    async fn test_shared_channel_factory() {
        let (factory, repository) = channel_factory::<TestSpec>(Arc::new(GnuId::new()));
        let id = GnuId::new();
        let valid_info = ValidChannelInfo {
            name: "Test Channel".to_string(),
            desc: "This is a test channel".to_string(),
            ..Default::default()
        };
        let valid_track = ValidTrackInfo {
            title: "Test Track".to_string(),
            creator: "Test Artist".to_string(),
            ..Default::default()
        };
        let config = ();

        let handle_get = repository.get_channel(&id);
        assert!(handle_get.is_none());

        let handle = factory.create_or_get(id.clone(), valid_info, valid_track, config.clone());

        // TestHhannel::newで非同期に初期化処理が行われるため、ここで一旦yieldしてから状態を確認する
        // TODO: 実際には、必ずしもspawnされたタスクが完了しているとは限らないため、適切な同期方法を検討する必要がある
        tokio::task::yield_now().await;

        assert_eq!(handle.id(), &id);
        assert_eq!(handle.config(), &config);
        assert!(handle.check_spawned());

        let handle2 = repository.get_channel(&id).unwrap();
        assert_eq!(handle.id(), handle2.id());

        let list_channels = repository.list_channels();
        assert_eq!(list_channels.len(), 1);
        assert_eq!(list_channels[0].id(), &id);

        let deleted_handle = repository.delete_channel(&id).unwrap();
        assert_eq!(handle.id(), deleted_handle.id());

        let deleted_handle2 = repository.delete_channel(&id);
        assert!(deleted_handle2.is_none());

        let list_channels = repository.list_channels();
        assert!(list_channels.is_empty());
    }
}
