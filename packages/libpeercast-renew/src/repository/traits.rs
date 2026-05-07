use std::fmt;

use peercast_gnuid::GnuId;

use crate::{
    model::{ChannelMeta, ValidChannelInfo, ValidTrackInfo},
    runtime,
};

/// Channelの管理を行うリポジトリのtrait
pub trait RepositorySpec {
    type Channel: Channel<Spec = Self>;
    type Config;
    type Handle: ChannelHandle<Spec = Self> + Clone + fmt::Debug;
    type Spawner: runtime::spawner::Spawner; //  tokio::runtime::Runtime;

    //
    type State;
    type Stats;

    //
    type Repository: ChannelRepository<Spec = Self> + Clone + std::fmt::Debug;
    // type Factory: ChannelFactory<Spec = Self>; // 定義しなくてよい
}

pub trait Channel: Sized {
    type Spec: RepositorySpec<Channel = Self>;
    // type Error;

    /// Channelの生成は非同期に行われることがあるため、async_runtimeを引数として受け取る
    /// ChannelFactory内で呼び出されることを想定しており、ロック状態でChannelインスタンスの生成が行われる
    /// そのためこの関数は軽量に保つべきである。
    fn new(
        spawner: &<Self::Spec as RepositorySpec>::Spawner,
        id: GnuId,
        valid_info: ValidChannelInfo,
        valid_track: ValidTrackInfo,
        config: <Self::Spec as RepositorySpec>::Config,
        manager: <Self::Spec as RepositorySpec>::Repository,
    ) -> Self;

    fn id(&self) -> &GnuId;
    fn handle(&self) -> <Self::Spec as RepositorySpec>::Handle;
}

pub trait ChannelHandle: Sized + Clone + fmt::Debug {
    type Spec: RepositorySpec<Handle = Self>;

    fn id(&self) -> &GnuId;
    fn config(&self) -> &<Self::Spec as RepositorySpec>::Config;

    fn state(&self) -> &<Self::Spec as RepositorySpec>::State;
    fn stats(&self) -> &<Self::Spec as RepositorySpec>::Stats;

    fn channel_meta(&self) -> ChannelMeta;
}

pub trait ChannelRepository {
    type Spec: RepositorySpec<Repository = Self>;

    fn get_channel(&self, id: &GnuId) -> Option<<Self::Spec as RepositorySpec>::Handle>;
    fn delete_channel(&self, id: &GnuId) -> Option<<Self::Spec as RepositorySpec>::Handle>;

    fn list_channels(&self) -> Vec<<Self::Spec as RepositorySpec>::Handle>;
}

/// Channelの生成を行うFactoryのtrait
/// usage:
///   let async_runtime = tokio::runtime::Handle::current();
///   let channel_handle = factory.create_or_get(async_runtime, id, config);
pub trait ChannelFactory {
    type Spec: RepositorySpec;

    fn self_session_id(&self) -> &GnuId;

    fn create_or_get(
        &self,
        id: GnuId,
        valid_info: ValidChannelInfo,
        valid_track: ValidTrackInfo,
        config: <Self::Spec as RepositorySpec>::Config,
    ) -> <Self::Spec as RepositorySpec>::Handle;
}
