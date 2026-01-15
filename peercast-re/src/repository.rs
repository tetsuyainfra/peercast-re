#![allow(dead_code)]

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use chrono::{DateTime, Utc};
use libpeercast_re::{
    pcp::{ChannelInfo, GnuId, TrackInfo},
    util::mutex_poisoned,
};
use tokio::sync::mpsc::UnboundedSender;

////////////////////////////////////////////////////////////////////////////////
// Structs and Traits
//
pub trait Channel {
    type Config;

    fn new(
        self_session_id: GnuId,
        channel_id: GnuId,
        channel_info: Option<ChannelInfo>,
        track_info: Option<TrackInfo>,
        rtmp_stream_sender: UnboundedSender<libpeercast_re::rtmp::stream_manager::StreamManagerMessage>,
        config: Option<Self::Config>,
    ) -> Self;

    // 本来ならこれ公開したくないよねぇ
    fn _start_channel_manager(&mut self) -> impl std::future::Future<Output = ()> + Send + '_ {
        async move {
            unimplemented!("you should implement start method in your Channel Procdureure");
        }
    }

    fn last_update(&self) -> DateTime<Utc>;

    fn before_delete(&mut self) {}

    fn id(&self) -> GnuId;
}

#[derive(Debug)]
pub struct ReChannelRepository<C> {
    impl_: Arc<Mutex<ImplRepository<C>>>,
}

////////////////////////////////////////////////////////////////////////////////
// ReChannelRepository
//
impl<C> Clone for ReChannelRepository<C> {
    fn clone(&self) -> Self {
        Self {
            impl_: Arc::clone(&self.impl_),
        }
    }
}

impl<C> ReChannelRepository<C>
where
    C: Channel + Clone + Send + Sync + 'static + std::fmt::Debug,
{
    /// 削除期限
    const DELETE_PERIOD_SEC: u64 = 300;
    /// 削除チェックのインターバル時間
    const DELETE_CHECK_INTERVAL_SEC: u64 = 60;

    pub fn new(session_id: &GnuId) -> Self {
        let impl_ = Arc::new(Mutex::new(ImplRepository::new(
            session_id,
            Self::DELETE_PERIOD_SEC,
            Self::DELETE_CHECK_INTERVAL_SEC,
        )));

        Self {
            impl_,
        }
    }

    #[inline]
    fn lock_impl(&self) -> std::sync::MutexGuard<'_, ImplRepository<C>> {
        self.impl_.lock().unwrap_or_else(mutex_poisoned)
    }

    #[inline]
    fn with_impl<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut ImplRepository<C>) -> R,
    {
        let mut guard = self.impl_.lock().unwrap_or_else(mutex_poisoned);
        f(&mut *guard)
    }

    pub fn session_id(&self) -> GnuId {
        self.lock_impl().session_id()
    }

    // HACKME: async 取り除きたいが・・・
    pub async fn create_or_get(
        &self,
        id: GnuId,
        channel_info: Option<ChannelInfo>,
        track_info: Option<TrackInfo>,
        rtmp_stream_manager: UnboundedSender<libpeercast_re::rtmp::stream_manager::StreamManagerMessage>,
        config: Option<C::Config>,
    ) -> C {
        let (mut ch, is_init) =
            self.with_impl(|s| s.create_or_get(id, channel_info, track_info, rtmp_stream_manager, config));
        // HACKME: ここで呼び出し順序の問題が発生する可能性あり
        // 二つのスレッドから同時にこれを呼び出してChannelを利用した場合、ChannelManagerがstartされてない可能性が残る
        // -> 完全に防ぐにはChannelRepository側でChannelの生成とstartを管理する必要があるが、
        //    tokio::sync::Mutexはコスト高いらしいので避けたい
        // -> 現状はChannelが持つChannelManagerへのSenderがUnboundedChannelなので、
        //    startされてなくてもメッセージ送信自体は可能なのでOKとする
        if is_init {
            ch._start_channel_manager().await;
        }
        ch
    }

    pub fn get(&self, id: &GnuId) -> Option<C> {
        self.with_impl(|s| s.get(id))
    }

    pub fn get_channels(&self) -> Vec<C> {
        self.with_impl(|s| s.get_channels())
    }

    pub fn delete(&mut self, id: &GnuId) -> bool {
        self.with_impl(|s| s.delete(id))
    }
}

#[derive(Debug)]
struct ImplRepository<C> {
    session_id: GnuId,
    channels: HashMap<GnuId, C>,
    delete_period_secs: u64,
    delete_check_interval_secs: u64,
}

impl<C> ImplRepository<C>
where
    C: Channel + Clone + Send + Sync + 'static + std::fmt::Debug,
{
    fn new(session_id: &GnuId, delete_period_secs: u64, delete_check_interval_secs: u64) -> Self {
        Self {
            session_id: session_id.clone(),
            channels: Default::default(),
            delete_period_secs,
            delete_check_interval_secs,
        }
    }

    fn session_id(&self) -> GnuId {
        self.session_id.clone()
    }

    fn create_or_get(
        &mut self,
        id: GnuId,
        channel_info: Option<ChannelInfo>,
        track_info: Option<TrackInfo>,
        rtmp_stream_manager: UnboundedSender<libpeercast_re::rtmp::stream_manager::StreamManagerMessage>,
        config: Option<C::Config>,
    ) -> (C, bool) {
        match self.channels.get(&id) {
            Some(ch) => (ch.clone(), false),
            None => {
                let ch =
                    C::new(self.session_id.clone(), id.clone(), channel_info, track_info, rtmp_stream_manager, config);
                tracing::info!("Created new channel: {:?}", ch);
                self.channels.insert(id.clone(), ch.clone());
                (ch, true)
            }
        }
    }

    fn get(&self, id: &GnuId) -> Option<C> {
        match self.channels.get(&id) {
            Some(ch) => Some(ch.clone()),
            None => None,
        }
    }

    fn get_channels(&self) -> Vec<C> {
        self.channels.iter().map(|(_id, ch)| ch.clone()).collect()
    }

    fn delete(&mut self, id: &GnuId) -> bool {
        match self.channels.remove(&id) {
            Some(mut ch) => {
                ch.before_delete();
                drop(ch);
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_repository_delete_task() {
        // ReRepositoryDeleteTask::new()
    }
}

////////////////////////////////////////////////////////////////////////////////
/// 新しいRepositoryのテスト
///
mod new_repository_test {

    use std::{
        any::{Any, TypeId},
        fmt::Debug,
    };

    use super::*;

    // Channelを複数型対応させるテスト用の新しいRepository
    // Configが型ごとに違う場合を想定している
    // そのためbuilderパターンを利用してChannel生成を行う
    trait Channel: Debug {
        type Config;
        // 異種のコンポーネントを同じコレクションに入れて、ランタイムに設定を流したいなら
        // object-safe なエントリポイントを用意しておくと便利

        fn configure(&mut self, cfg: &Self::Config) -> Result<(), &'static str>;
        fn get_config(&self) -> ();
    }

    trait ChannelBuilder {
        type Output: Channel;
        fn build(self, id: GnuId) -> Self::Output;
    }

    #[derive(Debug, Clone)]
    pub struct NewChannelRepository<T> {
        impl_: Arc<Mutex<NewImplRepository<T>>>,
    }

    #[derive(Debug)]
    struct NewImplRepository<T> {
        channels: HashMap<GnuId, T>,
    }

    impl<T> NewChannelRepository<T> {
        pub fn new() -> Self {
            Self {
                impl_: Arc::new(Mutex::new(NewImplRepository::new())),
            }
        }
    }

    impl<T> NewImplRepository<T> {
        fn new() -> Self {
            Self {
                channels: Default::default(),
            }
        }

        fn add_channel(&mut self, id: GnuId, builder: impl ChannelBuilder<Output = T>) {
            let channel = builder.build(id);
            // You need to generate or provide an id for the channel here
            // For example, if Channel has a method to get its id, you can use it
            // let id = channel.get_id();
            self.channels.insert(id, channel);
        }
    }

    #[derive(Debug, Clone, Default)]
    struct AConfig;
    #[derive(Debug, Clone)]
    struct AChannel(GnuId, AConfig);

    impl Channel for AChannel {
        type Config = AConfig;
        fn configure(&mut self, cfg: &Self::Config) -> Result<(), &'static str> {
            self.1 = cfg.clone();
            Ok(())
        }
        fn get_config(&self) -> () {
            todo!()
        }
    }
    impl AChannel {
        fn builder() -> AChannelBuilder {
            AChannelBuilder(AConfig)
        }
    }

    struct AChannelBuilder(AConfig);
    impl ChannelBuilder for AChannelBuilder {
        type Output = AChannel;
        fn build(self, id: GnuId) -> Self::Output {
            let ch = AChannel(id, self.0);
            ch
        }
    }

    #[derive(Debug, Clone, Default)]
    struct BConfig;

    #[derive(Debug, Clone)]
    struct BChannel(BConfig);

    impl BChannel {
        fn builder() -> BChannelBuilder {
            BChannelBuilder(BConfig)
        }
    }

    impl Channel for BChannel {
        type Config = BConfig;
        fn configure(&mut self, cfg: &Self::Config) -> Result<(), &'static str> {
            self.0 = cfg.clone();
            Ok(())
        }
        fn get_config(&self) -> () {
            todo!()
        }
    }

    struct BChannelBuilder(BConfig);
    impl ChannelBuilder for BChannelBuilder {
        type Output = BChannel;
        fn build(self, id: GnuId) -> Self::Output {
            let ch = BChannel(self.0);
            ch
        }
    }

    #[cfg(test)]
    mod t {
        use super::*;

        #[test]
        fn test_new_repository() {
            let mut repo = NewImplRepository::new();
            repo.add_channel(GnuId::new(), AChannel::builder());

            let mut repo = NewImplRepository::new();
            repo.add_channel(GnuId::new(), BChannel::builder());
        }
    }
}

mod a {
    use std::collections::HashMap;

    trait Component {
        fn run(&self);
    }

    trait Builder {
        fn build(&self) -> Box<dyn Component>;
    }

    #[derive(Clone)]
    struct AConfig {
        threads: usize,
    }

    struct A {
        cfg: AConfig,
    }
    impl Component for A {
        fn run(&self) {
            println!("A threads={}", self.cfg.threads)
        }
    }

    struct ABuilder {
        cfg: AConfig,
    }
    impl Builder for ABuilder {
        fn build(&self) -> Box<dyn Component> {
            Box::new(A {
                cfg: self.cfg.clone(),
            })
        }
    }

    #[derive(Clone)]
    struct BConfig {
        path: String,
    }

    struct B {
        cfg: BConfig,
    }
    impl Component for B {
        fn run(&self) {
            println!("B path={}", self.cfg.path)
        }
    }

    struct BBuilder {
        cfg: BConfig,
    }
    impl Builder for BBuilder {
        fn build(&self) -> Box<dyn Component> {
            Box::new(B {
                cfg: self.cfg.clone(),
            })
        }
    }

    struct Store {
        items: HashMap<u32, Box<dyn Component>>,
    }

    impl Store {
        fn add_from(&mut self, builder: impl Builder) {
            // self.items.push(builder.build());
            self.items.insert(0, builder.build());
        }
    }

    // trait ErasedChannel {
    //     fn configure_any(&mut self, cfg: &dyn Any) -> Result<(), &'static str>;
    //     fn config_type_id(&self) -> TypeId;
    // }
    // // すべての Component 実装に対して、設定型消去版を自動実装
    // impl<T: Channel> ErasedChannel for T {
    //     fn configure_any(&mut self, cfg: &dyn Any) -> Result<(), &'static str> {
    //         if let Some(c) = cfg.downcast_ref::<T::Config>() {
    //             T::configure(self, c)
    //         } else {
    //             Err("invalid config type")
    //         }
    //     }

    //     fn config_type_id(&self) -> TypeId {
    //         TypeId::of::<T::Config>()
    //     }
    // }
}
