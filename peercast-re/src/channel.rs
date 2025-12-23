use std::{
    net::SocketAddr,
    sync::{Arc, Mutex, RwLock},
};

use chrono::{DateTime, Utc};
use libpeercast_re::{
    pcp::{ChannelInfo, GnuId, TrackInfo},
    util::rwlock_read_poisoned,
};

use crate::repository::Channel;

#[allow(unused)]
#[derive(Clone, Debug)]
pub struct ReChannel {
    cid: GnuId,
    channel_info: Arc<RwLock<ChannelInfo>>,
    track_info: Arc<RwLock<TrackInfo>>,
    number_of_listener: Arc<RwLock<i32>>,
    number_of_relay: Arc<RwLock<i32>>,
    last_update: Arc<Mutex<DateTime<Utc>>>,
    created_at: Arc<DateTime<Utc>>,
    config: Option<ReConfig>,
    self_session_id: GnuId,
}

#[derive(Clone, Debug)]
pub struct ReConfig {
    pub tracker_ip: Option<SocketAddr>,
}

impl Channel for ReChannel {
    type Config = ReConfig;

    fn new(
        self_session_id: GnuId,
        channel_id: GnuId,
        channel_info: Option<libpeercast_re::pcp::ChannelInfo>,
        track_info: Option<libpeercast_re::pcp::TrackInfo>,
        config: Option<Self::Config>,
    ) -> Self {
        let now_ = Utc::now();

        Self {
            cid: channel_id,
            channel_info: RwLock::new(channel_info.unwrap_or_default()).into(),
            track_info: RwLock::new(track_info.unwrap_or_default()).into(),
            number_of_listener: RwLock::new(0).into(),
            number_of_relay: RwLock::new(0).into(),
            last_update: Arc::new(Mutex::new(now_.clone())),
            created_at: Arc::new(now_),
            config,
            self_session_id,
        }
    }

    fn last_update(&self) -> chrono::DateTime<chrono::Utc> {
        todo!()
    }
}

impl ReChannel {
    pub fn id(&self) -> GnuId {
        self.cid
    }

    pub fn channel_info(&self) -> ChannelInfo {
        self.channel_info.read().unwrap_or_else(rwlock_read_poisoned).clone()
    }
    pub fn track_info(&self) -> TrackInfo {
        self.track_info.read().unwrap_or_else(rwlock_read_poisoned).clone()
    }

    pub fn number_of_listener(&self) -> i32 {
        self.number_of_listener.read().unwrap_or_else(rwlock_read_poisoned).clone()
    }
    pub fn number_of_relay(&self) -> i32 {
        self.number_of_relay.read().unwrap_or_else(rwlock_read_poisoned).clone()
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at.as_ref().clone()
    }
}
