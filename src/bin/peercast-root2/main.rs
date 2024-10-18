use std::{
    collections::HashMap,
    marker::PhantomData,
    sync::{Arc, Mutex, RwLock},
    time::Duration,
};

use peercast_re::{
    pcp::GnuId,
    util::{mutex_poisoned, rwlock_read_poisoned, rwlock_write_poisoned},
};
use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tracing::info;

use crate::channel::{tracker_channel::TrackerChannel, ChannelStore};

mod channel;
mod manager;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let mut store: ChannelStore<TrackerChannel> = ChannelStore::new(None, None);

    info!("logging");
}
