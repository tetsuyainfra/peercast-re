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

    // 本来ならこれ後悔したくないよねぇ
    fn _start_channel_manager(&mut self) -> impl std::future::Future<Output = ()> + Send + '_ {
        async move {
            unimplemented!("you should implement start method in your Channel Procdureure");
        }
    }

    fn last_update(&self) -> DateTime<Utc>;

    fn before_delete(&mut self) {}

    fn id(&self) -> GnuId;
}

pub struct ReChannelRepository<C> {
    impl_: Arc<ImplRepository<C>>,
}

////////////////////////////////////////////////////////////////////////////////
// ReChannelRepository
//
impl<C> ReChannelRepository<C>
where
    C: Channel + Clone + Send + Sync + 'static + std::fmt::Debug,
{
    /// 削除期限
    const DELETE_PERIOD_SEC: u64 = 300;
    /// 削除チェックのインターバル時間
    const DELETE_CHECK_INTERVAL_SEC: u64 = 60;

    pub fn new(session_id: &GnuId) -> Self {
        let impl_ = Arc::new(ImplRepository::new(session_id, Self::DELETE_PERIOD_SEC, Self::DELETE_CHECK_INTERVAL_SEC));

        Self {
            impl_,
        }
    }

    pub fn session_id(&self) -> GnuId {
        self.impl_.session_id.clone()
    }

    // HACKME: async 取り除きたい
    pub async fn create_or_get(
        &self,
        id: GnuId,
        channel_info: Option<ChannelInfo>,
        track_info: Option<TrackInfo>,
        config: Option<C::Config>,
    ) -> C {
        let (mut ch, is_init) = self.impl_.create_or_get(id, channel_info, track_info, config);
        if is_init {
            ch._start_channel_manager().await;
        }
        ch
    }

    pub fn get(&self, id: &GnuId) -> Option<C> {
        self.impl_.get(id)
    }

    pub fn get_channels(&self) -> Vec<C> {
        self.impl_.get_channels()
    }

    pub fn delete(&mut self, id: &GnuId) -> bool {
        self.impl_.delete(id)
    }

    // ------------------------------------------------------------------------------
    // Misc functions
    //
    // pub fn channels_map(&self, func: fn(channels: &HashMap<GnuId, C>)) {
    //     // let mut lock = self.channels.lock().unwrap();
    //     func(&(self.channels));
    // }

    pub fn map_collect<F, R>(&self, func: F) -> Vec<R>
    where
        F: FnMut((&GnuId, &C)) -> R,
    {
        self.impl_.map_collect(func)
    }
}

struct ImplRepository<C> {
    session_id: GnuId,
    channels: Mutex<HashMap<GnuId, C>>,
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
    fn create_or_get(
        &self,
        id: GnuId,
        channel_info: Option<ChannelInfo>,
        track_info: Option<TrackInfo>,
        config: Option<C::Config>,
    ) -> (C, bool) {
        let mut channels = self.channels.lock().unwrap_or_else(mutex_poisoned);
        match channels.get(&id) {
            Some(ch) => (ch.clone(), false),
            None => {
                let ch = C::new(self.session_id.clone(), id.clone(), channel_info, track_info, config);
                tracing::info!("Created new channel: {:?}", ch);
                channels.insert(id.clone(), ch.clone());
                (ch, true)
            }
        }
    }

    fn get(&self, id: &GnuId) -> Option<C> {
        match self.channels.lock().unwrap_or_else(mutex_poisoned).get(&id) {
            Some(ch) => Some(ch.clone()),
            None => None,
        }
    }

    fn get_channels(&self) -> Vec<C> {
        self.channels.lock().unwrap_or_else(mutex_poisoned).iter().map(|(_id, ch)| ch.clone()).collect()
    }

    fn delete(&self, id: &GnuId) -> bool {
        match self.channels.lock().unwrap_or_else(mutex_poisoned).remove(&id) {
            Some(mut ch) => {
                ch.before_delete();
                drop(ch);
                true
            }
            None => false,
        }
    }

    fn map_collect<F, R>(&self, func: F) -> Vec<R>
    where
        F: FnMut((&GnuId, &C)) -> R,
    {
        // let channels = self.channels.lock().unwrap_or_else(mutex_poisoned);
        self.channels.lock().unwrap_or_else(mutex_poisoned).iter().map(func).collect()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_repository_delete_task() {
        // ReRepositoryDeleteTask::new()
    }
}
