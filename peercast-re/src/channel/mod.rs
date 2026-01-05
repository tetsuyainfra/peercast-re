use std::{
    net::SocketAddr,
    pin::Pin,
    sync::{Arc, Mutex},
};

use anyhow::Context;
use chrono::{DateTime, Utc};
use libpeercast_re::{
    pcp::{ChannelInfo, GnuId, TrackInfo},
    util::mutex_poisoned,
};

use crate::prelude::*;
use crate::repository::Channel;

mod manager;
mod stream;

#[allow(unused)]
#[derive(Debug)]
pub struct ReChannel {
    impl_: Arc<Mutex<ImplRechannel>>,
}

#[derive(Clone, Debug)]
pub struct ReConfig {
    pub tracker_ip: Option<SocketAddr>,
}

impl Clone for ReChannel {
    fn clone(&self) -> Self {
        Self {
            impl_: Arc::clone(&self.impl_),
        }
    }
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
            impl_: Arc::new(Mutex::new(impl_)),
        }
    }

    fn _start_channel_manager(&mut self) -> impl std::future::Future<Output = ()> + Send + '_ {
        {
            let re_channel = Clone::clone(self);
            let old_manager_status = self.with_impl(|s| std::mem::replace(&mut s.manager_status, ChMgrStatus::Running));

            if let ChMgrStatus::Standby(rx) = old_manager_status {
                let channel_manager = manager::ChannelManager::new(rx);
                return async {
                    let name = format!("ChManager-{}", re_channel.id());
                    let _ = tokio::task::Builder::new().name(&name).spawn(channel_manager.start(re_channel));
                };
            } else {
                unreachable!("Channel Manager status corrupted");
            }
        }
    }

    fn last_update(&self) -> chrono::DateTime<chrono::Utc> {
        todo!()
    }

    fn id(&self) -> GnuId {
        self.with_impl(|s| s.cid)
    }
}

impl ReChannel {
    #[inline]
    fn with_impl<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut ImplRechannel) -> R,
    {
        let mut guard = self.impl_.lock().unwrap_or_else(mutex_poisoned);
        f(&mut *guard)
    }

    pub fn channel_info(&self) -> ChannelInfo {
        self.with_impl(|s| s.channel_info.clone())
    }
    pub fn track_info(&self) -> TrackInfo {
        self.with_impl(|s| s.track_info.clone())
    }

    pub fn number_of_listener(&self) -> i32 {
        self.with_impl(|s| s.number_of_listener.clone())
    }
    pub fn number_of_relay(&self) -> i32 {
        self.with_impl(|s| s.number_of_relay.clone())
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.with_impl(|s| s.created_at.clone())
    }

    // 操作関係
    // チャンネルにTrackerIPを通知する
    pub async fn notify_tracker_ip(&self, tracker_ip: SocketAddr) -> anyhow::Result<()> {
        unimplemented!()
    }

    // ソースストリームを追加する
    pub async fn add_source_stream(&self, src_addr: &str) -> anyhow::Result<()> {
        let _ = self
            .with_impl(|s| s.add_source_stream(src_addr))
            .with_context(|| format!("Failed to add source stream cid: {}", self.id()))?;
        Ok(())
    }

    // 視聴関係
    pub async fn channel_stream(&self) -> anyhow::Result<stream::ReStream> {
        let r = self.with_impl(|s| s.create_stream())?;
        let x = r.await;

        Ok(x)
    }
}

#[derive(Debug)]
struct ImplRechannel {
    cid: GnuId,
    channel_info: ChannelInfo,
    track_info: TrackInfo,
    number_of_listener: i32,
    number_of_relay: i32,
    last_update: DateTime<Utc>,
    created_at: DateTime<Utc>,
    config: Option<ReConfig>,
    self_session_id: GnuId,

    // Channel Manager関連
    sender: manager::ChannelManagerSender,
    manager_status: ChMgrStatus,
}

#[derive(Debug)]
enum ChMgrStatus {
    Standby(tokio::sync::mpsc::UnboundedReceiver<manager::Message>),
    Running,
}

impl ImplRechannel {
    fn new(
        self_session_id: GnuId,
        channel_id: GnuId,
        channel_info: Option<ChannelInfo>,
        track_info: Option<TrackInfo>,
        config: Option<ReConfig>,
    ) -> Self {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        Self {
            cid: channel_id,
            channel_info: channel_info.unwrap_or_default(),
            track_info: track_info.unwrap_or_default(),
            number_of_listener: 0,
            number_of_relay: 0,
            last_update: Utc::now(),
            created_at: Utc::now(),
            config,
            self_session_id,
            // Channel Manager関連
            sender: tx,
            manager_status: ChMgrStatus::Standby(rx),
        }
    }

    fn add_source_stream(&self, src_addr: &str) -> anyhow::Result<()> {
        let _ = self
            .sender
            .send(manager::Message::AddSourceStream(src_addr.to_string()))
            .with_context(|| format!("Failed send AddSourceStream message cid: {}", self.cid))?;
        Ok(())
    }

    fn create_stream(
        &self,
    ) -> anyhow::Result<Pin<Box<dyn std::future::Future<Output = stream::ReStream> + Send + 'static>>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let _ = self
            .sender
            .send(manager::Message::AddSubscriber(tx))
            .with_context(|| format!("Failed send AddSubscriber message cid: {}", self.cid))?;

        let cid = self.cid;
        Ok(Box::pin(async move {
            let watcher = rx.await;
            debug!("Watcher received for cid: {}", cid);

            let stream = stream::ReStream::new(cid).await;
            stream
        }))
    }
}
