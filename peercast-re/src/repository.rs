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
        config: Option<C::Config>,
    ) -> C {
        let (mut ch, is_init) = self.with_impl(|s| s.create_or_get(id, channel_info, track_info, config));
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
        config: Option<C::Config>,
    ) -> (C, bool) {
        match self.channels.get(&id) {
            Some(ch) => (ch.clone(), false),
            None => {
                let ch = C::new(self.session_id.clone(), id.clone(), channel_info, track_info, config);
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
