use serde::de;

use crate::pcp::{ChannelInfo, GnuId, TrackInfo};

use super::Channel;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DummyChannelConfig {
    session_id: GnuId,
}

impl DummyChannelConfig {
    pub fn new() -> Self {
        DummyChannelConfig {
            session_id: GnuId::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DummyChannel {
    id: GnuId,
    config: DummyChannelConfig,
    channel_info: Option<ChannelInfo>,
    track_info: Option<TrackInfo>,
}

impl Channel for DummyChannel {
    type Config = DummyChannelConfig;

    fn new(
        id: crate::pcp::GnuId,
        channel_info: Option<ChannelInfo>,
        track_info: Option<TrackInfo>,
        config: Option<Self::Config>,
    ) -> Self {
        DummyChannel {
            id,
            config: config.unwrap_or_else(|| DummyChannelConfig::new()),
            channel_info,
            track_info,
        }
    }

    fn cid(&self) -> GnuId {
        self.id
    }

    fn after_create(&mut self) -> impl std::future::Future<Output = ()> + Send {
        async {}
    }

    fn state(&self) -> super::ChannelState {
        todo!()
    }

    fn channel_type(&self) -> super::ChannelType {
        todo!()
    }

    fn config(&self) -> Option<&Self::Config> {
        todo!()
    }

    fn update_config(&mut self, config: Self::Config) {
        todo!()
    }

    fn tracker_address(&self) -> Option<std::net::SocketAddr> {
        todo!()
    }

    fn channel_info(&self) -> Option<ChannelInfo> {
        todo!()
    }

    fn track_info(&self) -> Option<TrackInfo> {
        todo!()
    }

    fn number_of_listener(&self) -> i32 {
        todo!()
    }

    fn number_of_relay(&self) -> i32 {
        todo!()
    }

    fn created_at(&self) -> chrono::DateTime<chrono::Utc> {
        todo!()
    }

    fn updated_at(&self) -> chrono::DateTime<chrono::Utc> {
        todo!()
    }

    fn viewed_at(&self) -> chrono::DateTime<chrono::Utc> {
        todo!()
    }
}
