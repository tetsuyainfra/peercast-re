use peercast_re::pcp::GnuId;

use crate::store::Channel;

#[derive(Debug, Clone)]
pub struct RootChannelConfig {
    pub remote_session_id: GnuId,
    pub remote_broadcast_id: GnuId,
}

#[derive(Debug, Clone)]
pub struct RootChannel {
    id: GnuId,
    config: RootChannelConfig,
}

impl Channel for RootChannel {
    type Config = RootChannelConfig;

    fn new(id: peercast_re::pcp::GnuId, config: Self::Config) -> Self {
        Self { id, config }
    }
}

#[cfg(test)]
mod t {
    use crate::test_helper::*;

    use super::RootChannel;

    #[test]
    fn type_check() {
        is_send::<RootChannel>();
        is_sync::<RootChannel>();
    }
}
