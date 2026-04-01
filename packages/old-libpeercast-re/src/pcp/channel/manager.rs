use std::{
    collections::HashMap,
    sync::{atomic::AtomicI32, Arc, Mutex},
};

use tracing::info;

use crate::pcp::GnuId;

use super::{channel::ChannelType, Channel, ChannelInfo, TrackInfo};

// MEMO: ChannelをArcで返す方が良いかも
// というのも下手にインスタンスで所持していて操作をされたら、
// その瞬間に有効になっているチャンネルを操作されると困る
pub struct ChannelManager {
    session_id: GnuId,
    channels: Arc<Mutex<HashMap<GnuId, Channel>>>,
}

impl ChannelManager {
    pub fn new(session_id: &GnuId) -> Arc<ChannelManager> {
        Arc::new(ChannelManager {
            session_id: session_id.clone(),
            channels: Default::default(),
        })
    }

    pub fn session_id(&self) -> GnuId {
        self.session_id.clone()
    }

    pub fn channels_lock(&self, func: fn(channels: &mut HashMap<GnuId, Channel>)) {
        let mut lock = self.channels.lock().unwrap();
        func(&mut (*lock));
    }

    pub fn create(
        &self,
        id: GnuId,
        ch_type: ChannelType,
        channel_info: Option<ChannelInfo>,
        track_info: Option<TrackInfo>,
    ) -> Option<Channel> {
        let mut channels = match self.channels.lock() {
            Ok(c) => c,
            Err(_) => todo!(),
        };

        let channel = Channel::new(self.session_id, id, ch_type, channel_info, track_info);
        match channels.insert(id, channel) {
            Some(old_ch) => {
                channels.insert(id, old_ch);
                None
            }
            None => {
                let ch = channels.get(&id).unwrap().clone();
                info!("created channels. {:?}", &ch);
                Some(ch)
            }
        }
    }
    pub fn create_or_get(
        &self,
        id: GnuId,
        ch_type: ChannelType,
        channel_info: Option<ChannelInfo>,
        track_info: Option<TrackInfo>,
    ) -> Channel {
        let mut channels = match self.channels.lock() {
            Ok(c) => c,
            Err(_) => todo!(),
        };
        match channels.get(&id) {
            Some(ch) => ch.clone(),
            None => {
                // channelが無かった場合
                let channel = Channel::new(self.session_id, id, ch_type, channel_info, track_info);
                match channels.insert(id, channel) {
                    Some(id) => panic!("ChannelManager have same GnuID. {:?}", &self.channels),
                    None => {
                        let ch = channels.get(&id).unwrap().clone();
                        info!("created channels. {:?}", &ch);
                        ch
                    }
                }
            }
        }
    }

    pub fn delete(&self, id: &GnuId) -> bool {
        let mut channels = match self.channels.lock() {
            Ok(c) => c,
            Err(_) => todo!(),
        };
        match channels.remove(&id) {
            Some(_) => true,
            None => false,
        }
    }

    pub fn get(&self, id: &GnuId) -> Option<Channel> {
        let channels = match self.channels.lock() {
            Ok(c) => c,
            Err(_) => todo!(),
        };
        match channels.get(id).clone() {
            Some(ch) => Some(ch.clone()),
            None => None,
        }
    }

    #[allow(dead_code)]
    pub fn map_collect<F, R>(&self, func: F) -> Vec<R>
    where
        F: FnMut((&GnuId, &Channel)) -> R,
    {
        let channels = match self.channels.lock() {
            Ok(c) => c,
            Err(_) => todo!(),
        };
        channels.iter().map(func).collect()
    }
}

impl std::fmt::Debug for ChannelManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChannelManager")
            .field("channels", &self.channels)
            .finish()
    }
}

#[cfg(test)]
mod t {
    use super::*;

    #[crate::test]
    async fn test_channel_manager() {
        let mut manager = ChannelManager::new(&GnuId::new());
        let id = GnuId::new();
        let ch_type = ChannelType::Relay;
        // ("127.0.0.1:7144".parse().unwrap());
        let info = ChannelInfo::new();
        let ch1 = manager.create_or_get(id, ch_type, Some(info), Default::default());
        let ch1_2 = manager.get(&id);
    }
}
