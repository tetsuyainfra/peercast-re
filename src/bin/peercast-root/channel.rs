use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use chrono::{DateTime, Utc};
use peercast_re::{
    pcp::{decode::PcpBroadcast, GnuId},
    ConnectionId,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    manager::{RootManager, RootManagerMessage},
    TrackerConnection,
};
//------------------------------------------------------------------------------
// TrackerDetail
//
#[derive(Debug, Clone)]
pub struct TrackerDetail {}

//------------------------------------------------------------------------------
// ChannelTrait
//
pub trait ChannelTrait: Clone {
    type Config;
    fn new(
        self_session_id: Arc<GnuId>,
        self_broadcast_id: Arc<GnuId>,
        channel_id: Arc<GnuId>,
        config: Self::Config,
    ) -> Self;

    // call remove
    fn before_remove(&mut self);
}

//------------------------------------------------------------------------------
// ChannelManager
//

#[derive(Debug)]
pub struct ChannelManager<C: ChannelTrait> {
    session_id: Arc<GnuId>,
    broadcast_id: Arc<GnuId>,
    channels: Arc<RwLock<HashMap<GnuId, C>>>,
}
impl<C: ChannelTrait> Clone for ChannelManager<C> {
    fn clone(&self) -> Self {
        Self {
            session_id: Arc::clone(&self.session_id),
            broadcast_id: Arc::clone(&self.broadcast_id),
            channels: Arc::clone(&self.channels),
        }
    }
}

impl<C: ChannelTrait> ChannelManager<C> {
    pub fn new(self_session_id: Option<GnuId>, self_broadcast_id: Option<GnuId>) -> Self {
        let session_id = Arc::new(self_session_id.unwrap_or_else(|| GnuId::new()));
        let broadcast_id = Arc::new(self_broadcast_id.unwrap_or_else(|| GnuId::new()));
        Self {
            session_id,
            broadcast_id,
            channels: Default::default(),
        }
    }

    pub fn create_or_get(&self, channel_id: GnuId, config: C::Config) -> C {
        self.get(&channel_id)
            .or_else(|| {
                // Channelがなければロックしてから検索して返す
                let mut channels = self.channels.write().unwrap();
                let arc_channel_id = Arc::new(channel_id.clone());
                let ch = channels.entry(channel_id).or_insert_with(|| {
                    C::new(
                        self.session_id.clone(),
                        self.broadcast_id.clone(),
                        arc_channel_id,
                        config,
                    )
                });
                Some(ch.clone())
            })
            .unwrap()
    }

    pub fn get(&self, channel_id: &GnuId) -> Option<C> {
        self.channels
            .read()
            .unwrap()
            .get(channel_id)
            .map(|c| c.clone())
    }

    pub fn get_channels(&self) -> Vec<C> {
        self.channels
            .read()
            .unwrap()
            .values()
            .map(|c| c.clone())
            .collect()
    }

    pub fn remove(&mut self, channel_id: &GnuId) {
        let ch = self.channels.write().unwrap().remove(channel_id);
        ch.map(|mut c| c.before_remove());
    }
}

//------------------------------------------------------------------------------
// TrackerChannel
//

#[derive(Debug, Clone)]
pub struct TrackerChannelConfig {
    pub broadcast_id: Arc<GnuId>,
    pub first_broadcast: Arc<PcpBroadcast>,
}

#[derive(Debug, Clone)]
pub struct TrackerChannel {
    self_session_id: Arc<GnuId>,
    pub channel_id: Arc<GnuId>,
    broadcast: Arc<PcpBroadcast>,
    config: Arc<TrackerChannelConfig>,
    manager_sender: UnboundedSender<RootManagerMessage>,
    pub detail_reciever: tokio::sync::watch::Receiver<TrackerDetail>,

    pub created_at: Arc<DateTime<Utc>>,
    _called_before_remove: bool,
}

impl TrackerChannel {
    pub fn tracker_connection(
        &self,
        connection_id: ConnectionId,
        remote_broadcast_id: Arc<GnuId>,
        first_broadcast: Arc<PcpBroadcast>,
    ) -> TrackerConnection {
        TrackerConnection::new(
            connection_id,
            self.config.clone(),
            self.manager_sender.clone(),
            remote_broadcast_id,
            first_broadcast,
        )
    }
}

impl ChannelTrait for TrackerChannel {
    type Config = TrackerChannelConfig;

    fn new(
        self_session_id: Arc<GnuId>,
        _self_broadcast_id: Arc<GnuId>,
        channel_id: Arc<GnuId>,
        config: Self::Config,
    ) -> Self {
        let (detail_sender, detail_reciever) = tokio::sync::watch::channel(TrackerDetail {});

        // self_broadcast_idはいらないので無視する
        let manager_sender = RootManager::start(channel_id.clone(), detail_sender);

        TrackerChannel {
            channel_id,
            self_session_id,
            broadcast: config.first_broadcast.clone(),
            config: Arc::new(config),
            manager_sender,
            detail_reciever,

            created_at: Arc::new(Utc::now()),
            _called_before_remove: false,
        }
    }

    fn before_remove(&mut self) {
        if !self._called_before_remove {
            // 色々やる
        }
    }
}
