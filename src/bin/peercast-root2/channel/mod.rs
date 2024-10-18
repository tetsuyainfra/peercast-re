use std::{
    collections::HashMap,
    marker::PhantomData,
    sync::{Arc, Mutex, RwLock},
};

use peercast_re::{
    pcp::GnuId,
    util::{rwlock_read_poisoned, rwlock_write_poisoned},
};
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tracing::info;

pub use self::store::ChannelStore;
use self::store::ChannelWatcherMessage;

mod store;
pub mod tracker_channel;

//------------------------------------------------------------------------------
// ChannelTrait
//
pub trait ChannelTrait
where
    Self: Clone + Send + Sync,
    Self: PartialEq,
{
    // type InternalStore;
    type Config;
    fn new(
        root_session_id: GnuId,
        root_broadcast_id: GnuId,
        channel_id: GnuId,
        config: Self::Config,
        // status_sender: watch::Sender<ChannelStatus>, // store: Weak<Self::InternalStore>,
        watcher_sender: mpsc::UnboundedSender<ChannelWatcherMessage>,
    ) -> Self;

    // Stop channel
    fn stop(&self);

    // call before remove channel
    fn before_remove(&mut self, channel_id: GnuId) {
        info!(?channel_id, "will remove");
    }

    // call after remove channel
    fn after_remove(&mut self, channel_id: GnuId) {
        info!(?channel_id, "removed");
    }
}
