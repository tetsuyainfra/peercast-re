
use tracing::info;

use crate::channel::{tracker_channel::TrackerChannel, ChannelStore};

mod channel;
mod manager;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let store: ChannelStore<TrackerChannel> = ChannelStore::new(None, None);

    info!("logging");
}
