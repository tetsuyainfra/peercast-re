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
}
