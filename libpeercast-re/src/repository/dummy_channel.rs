use serde::de;

use crate::pcp::GnuId;

use super::Channel;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DummyChannelConfig {
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
pub(super) struct DummyChannel {
    id: GnuId,
    config: DummyChannelConfig,
}

impl Channel for DummyChannel {
    type Config = DummyChannelConfig;

    fn new(id: crate::pcp::GnuId, config: Option<Self::Config>) -> Self {
        DummyChannel {
            id,
            config: config.unwrap_or_else(|| DummyChannelConfig::new()),
        }
    }

    fn id(&self) -> GnuId {
        self.id
    }

    fn after_create(&mut self) -> impl std::future::Future<Output = ()> + Send {
        async {}
    }
}
