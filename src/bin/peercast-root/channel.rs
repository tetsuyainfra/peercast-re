use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use peercast_re::pcp::GnuId;

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
