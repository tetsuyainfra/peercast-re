use chrono::{DateTime, Utc};
use peercast_re::pcp::GnuId;
use tokio::sync::watch;

use crate::manager::RootManager;

use super::ChannelTrait;

//------------------------------------------------------------------------------
// TrackerChannel
//
#[derive(Debug, Clone)]
pub struct TrackerChannel {
    id: GnuId,
    root_session_id: GnuId,
    root_broadcast_id: GnuId,
    detail_receiver: watch::Receiver<ChannelDetail>,
    watcher_sender_: tokio::sync::mpsc::UnboundedSender<super::store::ChannelWatcherMessage>,
}

#[derive(Debug, Clone)]
pub struct TrackerChannelConfig {}

impl ChannelTrait for TrackerChannel {
    type Config = TrackerChannelConfig;

    fn new(
        root_session_id: GnuId,
        root_broadcast_id: GnuId,
        channel_id: GnuId,
        config: Self::Config,
        // status_sender: watch::Sender<ChannelStatus>, // store: Weak<Self::InternalStore>,
        watcher_sender_: tokio::sync::mpsc::UnboundedSender<super::store::ChannelWatcherMessage>,
    ) -> Self {
        let (detail_sender, detail_receiver) = watch::channel(ChannelDetail::new());
        let manager_sender = RootManager::start(channel_id.clone(), detail_sender);

        Self {
            id: channel_id,
            root_session_id,
            root_broadcast_id,
            detail_receiver,
            watcher_sender_,
        }
    }
    fn stop(&self) {}
}

impl PartialEq for TrackerChannel {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl TrackerChannel {
    fn _stop(&self) {}
}

//------------------------------------------------------------------------------
// ChannelStatus
//
pub enum ChannelControl {
    Stop,
}

//------------------------------------------------------------------------------
// ChannelDetail
//

#[derive(Debug, Clone)]
pub struct ChannelDetail {
    created_at: DateTime<Utc>,
}

impl ChannelDetail {
    fn new() -> Self {
        Self {
            created_at: DateTime::default(),
        }
    }
}
