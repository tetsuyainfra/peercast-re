use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, RwLock},
};

use chrono::{DateTime, Utc};
use clap::builder::Str;
use futures_util::future::select_all;
use peercast_re::{
    pcp::{
        decode::{PcpBroadcast, PcpChannelInfo, PcpTrackInfo},
        GnuId,
    },
    ConnectionId,
};
use serde::de;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tracing::{debug, info};

use crate::{
    manager::{RootManager, RootManagerMessage},
    TrackerConnection,
};

//------------------------------------------------------------------------------
// ChannelTrait
//
pub trait ChannelTrait: Clone + Send + Sync {
    type Config;
    fn new(
        root_session_id: Arc<GnuId>,
        root_broadcast_id: Arc<GnuId>,
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
pub struct ChannelStore<C: ChannelTrait> {
    // Root(つまり自分のId)
    root_session_id: Arc<GnuId>,
    root_broadcast_id: Arc<GnuId>,
    channels: Arc<RwLock<HashMap<GnuId, C>>>,
    // channel_keeper: ChannelStoreKeeper<C>,
}

impl<C: ChannelTrait> ChannelStore<C> {
    pub fn new(self_session_id: Option<GnuId>, self_broadcast_id: Option<GnuId>) -> Self {
        let root_session_id = Arc::new(self_session_id.unwrap_or_else(|| GnuId::new()));
        let root_broadcast_id = Arc::new(self_broadcast_id.unwrap_or_else(|| GnuId::new()));
        // let channels = Default::default();
        Self {
            root_session_id,
            root_broadcast_id,
            channels: Default::default(),
            // channels: channels.clone(),
            // channel_keeper: ,
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
                        self.root_session_id.clone(),
                        self.root_broadcast_id.clone(),
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
/// TrackerChannelを作成する際に指定する構造物
///
#[derive(Debug, Clone)]
pub struct TrackerChannelConfig {
    pub tracker_session_id: Arc<GnuId>,
    pub tracker_broadcast_id: Arc<GnuId>,
    pub first_broadcast: Arc<PcpBroadcast>,
}

//------------------------------------------------------------------------------
/// Channelを扱うための構造体
///
#[derive(Debug, Clone)]
pub struct TrackerChannel {
    root_session_id: Arc<GnuId>,
    root_broadcast_id: Arc<GnuId>,
    pub channel_id: Arc<GnuId>,
    // broadcast: Arc<PcpBroadcast>,
    config: Arc<TrackerChannelConfig>,
    manager_sender: Arc<UnboundedSender<RootManagerMessage>>,
    detail_receiver: Arc<tokio::sync::watch::Receiver<TrackerDetail>>,

    pub created_at: Arc<DateTime<Utc>>,
    // _called_before_remove: bool,
}

//------------------------------------------------------------------------------
// TrackerChannel : Channelを扱うための構造体
//
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
            self.manager_sender.as_ref().clone(),
            remote_broadcast_id,
            first_broadcast,
        )
    }

    pub fn detail(&self) -> TrackerDetail {
        self.detail_receiver.borrow().clone()
    }
}

impl ChannelTrait for TrackerChannel {
    type Config = TrackerChannelConfig;

    fn new(
        root_session_id: Arc<GnuId>,
        root_broadcast_id: Arc<GnuId>,
        channel_id: Arc<GnuId>,
        config: Self::Config,
    ) -> Self {
        let created_at = Arc::new(Utc::now());
        let mut detail = TrackerDetail::new(channel_id.clone());
        detail.created_at = created_at.clone();

        let (detail_sender, detail_receiver) = tokio::sync::watch::channel(detail);
        // self_broadcast_idはいらないので無視する
        let manager_sender = RootManager::start(channel_id.clone(), detail_sender);

        TrackerChannel {
            channel_id,
            root_session_id,
            root_broadcast_id,
            config: Arc::new(config),
            manager_sender: Arc::new(manager_sender),
            detail_receiver: Arc::new(detail_receiver),

            created_at,
            // _called_before_remove: false,
        }
    }

    fn before_remove(&mut self) {
        info!(
            "TrackerChannel({}) will be removed.",
            self.channel_id.as_ref()
        );
        //     if !self._called_before_remove {
        //         // 色々やる
        //     }
    }
}

//------------------------------------------------------------------------------
// TrackerDetail : チェンネル詳細
// TODO: そのうちlibに移す。
// TODO: PcpBroadcastから変換できるようにする？
#[derive(Debug, Clone)]
pub struct TrackerDetail {
    pub channel_info: PcpChannelInfo,
    pub track_info: PcpTrackInfo,
    //
    pub created_at: Arc<DateTime<Utc>>,
    pub id: Arc<GnuId>,
}

impl TrackerDetail {
    pub fn new(id: Arc<GnuId>) -> Self {
        TrackerDetail {
            channel_info: Default::default(),
            track_info: Default::default(),
            created_at: Default::default(),
            id,
        }
    }

    pub fn id(&self) -> GnuId {
        self.id.as_ref().clone()
    }

    // pub fn name(&self) -> String {
    //     if let Some(ref name) = self.channel_info.name {
    //         name.clone()
    //     } else {
    //         String::from("")
    //     }
    // }

    // getter!(&self, channel_info, typ);
    // getter!(&self, channel_info, name);
    // getter!(&self, channel_info, genre);
    // getter!(&self, channel_info, desc);
    // getter!(&self, channel_info, comment);
    // getter!(&self, channel_info, url);
    // getter!(&self, channel_info, stream_ext);
    // getter!(&self, channel_info, stream_type);
    // getter!(&self, channel_info, bitrate, i32, 0);
}
