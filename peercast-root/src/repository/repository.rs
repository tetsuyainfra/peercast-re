use std::sync::{Arc, RwLock};

use libpeercast_re::{
    GnuId,
    model::{ChannelMeta, ValidChannelInfo, ValidTrackInfo},
    repository::{
        shared::{SharedChannelFactory, SharedChannelRepository},
        traits::*,
    },
    runtime::spawner::{Spawner, TokioSpawner},
    utils::sync::rwlock_read_poisoned,
};

pub type RootChannelRepository = SharedChannelRepository<RootChannelSpec>;
pub type RootChannelFactory = SharedChannelFactory<RootChannelSpec>;

#[derive(Debug)]
pub struct RootChannelSpec;

impl RepositorySpec for RootChannelSpec {
    type Channel = RootChannel;
    type Config = RootChannelConfig;

    type Handle = RootChannelHandle;

    type Spawner = TokioSpawner;

    type State = ();

    type Stats = ();

    type Repository = SharedChannelRepository<Self>;
}

#[derive(Debug)]
pub struct RootChannel {
    id: GnuId,
    manager: SharedChannelRepository<RootChannelSpec>,
    valid_info: Arc<RwLock<ValidChannelInfo>>,
    valid_track: Arc<RwLock<ValidTrackInfo>>,
    config: Arc<RootChannelConfig>,
    // 非同期に初期化処理を行うためのフィールド
    spawned: tokio::task::JoinHandle<()>,
}

impl Channel for RootChannel {
    type Spec = RootChannelSpec;

    fn new(
        spawner: &<Self::Spec as RepositorySpec>::Spawner,
        id: libpeercast_re::GnuId,
        valid_info: ValidChannelInfo,
        valid_track: ValidTrackInfo,
        config: <Self::Spec as RepositorySpec>::Config,
        manager: <Self::Spec as RepositorySpec>::Repository,
    ) -> Self {
        let spawned = spawner.spawn(async {
            // ここで非同期に初期化処理を行うことができる
        });

        Self {
            id,
            manager,
            config: Arc::new(config),
            valid_info: Arc::new(RwLock::new(valid_info)),
            valid_track: Arc::new(RwLock::new(valid_track)),
            //
            spawned,
        }
    }

    fn id(&self) -> &libpeercast_re::GnuId {
        todo!()
    }

    fn handle(&self) -> <Self::Spec as RepositorySpec>::Handle {
        RootChannelHandle {
            id: self.id.clone(),
            config: Arc::clone(&self.config),
            valid_info: Arc::clone(&self.valid_info),
            valid_track: Arc::clone(&self.valid_track),
        }
    }
}

#[derive(Debug)]
pub struct RootChannelConfig {
    pub broadcast_id: Option<GnuId>,
    pub tracker_addr: Option<std::net::SocketAddr>,
}

#[derive(Debug, Clone)]
pub struct RootChannelHandle {
    id: GnuId,
    valid_info: Arc<RwLock<ValidChannelInfo>>,
    valid_track: Arc<RwLock<ValidTrackInfo>>,
    config: Arc<RootChannelConfig>,
}

impl ChannelHandle for RootChannelHandle {
    type Spec = RootChannelSpec;

    fn id(&self) -> &GnuId {
        &self.id
    }

    fn config(&self) -> &<Self::Spec as RepositorySpec>::Config {
        todo!()
    }

    fn state(&self) -> &<Self::Spec as RepositorySpec>::State {
        todo!()
    }

    fn stats(&self) -> &<Self::Spec as RepositorySpec>::Stats {
        todo!()
    }

    fn channel_meta(&self) -> ChannelMeta {
        let valid_info = self.valid_info.read().unwrap_or_else(rwlock_read_poisoned).clone();
        let valid_track = self.valid_track.read().unwrap_or_else(rwlock_read_poisoned).clone();
        let meta = ChannelMeta::with_valid(self.id, valid_info, valid_track);
        // TODO:実装街
        // meta.tracker_addr = self.config.tracker_addr;
        // meta.number_of_listener = self.config.number_of_listener;
        // meta.number_of_relay = self.config.number_of_relay;A
        // meta.created_at = self.config.created_at;
        // meta.namespace = self.config.namespace.clone();

        meta
    }
}
