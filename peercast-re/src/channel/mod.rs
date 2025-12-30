use std::{
    net::SocketAddr,
    sync::{Arc, Mutex, RwLock},
};

use chrono::{DateTime, Utc};
use libpeercast_re::{
    pcp::{ChannelInfo, GnuId, TrackInfo},
    util::{mutex_poisoned, rwlock_read_poisoned},
};

use crate::prelude::*;
use crate::repository::Channel;

#[allow(unused)]
#[derive(Clone, Debug)]
pub struct ReChannel {
    impl_: Arc<ImplRechannel>,
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
        channel_info: Option<ChannelInfo>,
        track_info: Option<TrackInfo>,
        config: Option<Self::Config>,
    ) -> Self {
        let impl_ = ImplRechannel::new(self_session_id, channel_id, channel_info, track_info, config);
        Self {
            impl_: Arc::new(impl_),
        }
    }

    fn _start_channel_manager(&mut self) -> impl std::future::Future<Output = ()> + Send + '_ {
        let cid = self.impl_.cid.clone();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        self.impl_.sender.lock().unwrap_or_else(mutex_poisoned).replace(tx);

        async move {
            let _ = tokio::spawn(async move {
                info!("Starting Channel Manager: {:?}", cid);
                let r = rx.recv().await;
                info!("Stopping Channel Manager: {:?} {:?}", cid, r);
            });
        }
    }

    fn last_update(&self) -> chrono::DateTime<chrono::Utc> {
        todo!()
    }

    fn id(&self) -> GnuId {
        self.impl_.cid.clone()
    }
}

impl ReChannel {
    pub fn channel_info(&self) -> ChannelInfo {
        self.impl_.channel_info.read().unwrap_or_else(rwlock_read_poisoned).clone()
    }
    pub fn track_info(&self) -> TrackInfo {
        self.impl_.track_info.read().unwrap_or_else(rwlock_read_poisoned).clone()
    }

    pub fn number_of_listener(&self) -> i32 {
        self.impl_.number_of_listener.read().unwrap_or_else(rwlock_read_poisoned).clone()
    }
    pub fn number_of_relay(&self) -> i32 {
        self.impl_.number_of_relay.read().unwrap_or_else(rwlock_read_poisoned).clone()
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.impl_.created_at.as_ref().clone()
    }

    // 操作関係
    // チャンネルにTrackerIPを通知する

    // 視聴関係
    // pub fn channel_stream(&self) -> Result<stream::ReStream, std::io::Error> {
    //     let stream = stream::ReStream::new(self.cid);

    //     Ok(stream)
    // }
}

#[derive(Debug)]
struct ImplRechannel {
    cid: GnuId,
    channel_info: Arc<RwLock<ChannelInfo>>,
    track_info: Arc<RwLock<TrackInfo>>,
    number_of_listener: Arc<RwLock<i32>>,
    number_of_relay: Arc<RwLock<i32>>,
    last_update: Arc<Mutex<DateTime<Utc>>>,
    created_at: Arc<DateTime<Utc>>,
    config: Option<ReConfig>,
    self_session_id: GnuId,

    sender: Arc<Mutex<Option<tokio::sync::mpsc::UnboundedSender<()>>>>,
}

impl ImplRechannel {
    fn new(
        self_session_id: GnuId,
        channel_id: GnuId,
        channel_info: Option<ChannelInfo>,
        track_info: Option<TrackInfo>,
        config: Option<ReConfig>,
    ) -> Self {
        Self {
            cid: channel_id,
            channel_info: RwLock::new(channel_info.unwrap_or_default()).into(),
            track_info: RwLock::new(track_info.unwrap_or_default()).into(),
            number_of_listener: RwLock::new(0).into(),
            number_of_relay: RwLock::new(0).into(),
            last_update: Arc::new(Mutex::new(Utc::now())),
            created_at: Arc::new(Utc::now()),
            config,
            self_session_id,
            sender: Arc::new(Mutex::new(None)),
        }
    }
}

struct ChannelManager {}
