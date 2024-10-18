use peercast_re::pcp::GnuId;
use peercast_re_api::models::ChannelStatus;
use tokio::sync::{mpsc, watch};
use tracing::info;

use crate::channel::tracker_channel::{ChannelControl, ChannelDetail};

pub struct RootManager {
    channel_id: GnuId,
}

impl RootManager {
    pub fn start(
        channel_id: GnuId,
        detail_sender: watch::Sender<ChannelDetail>,
        // ) -> mpsc::UnboundedSender<RootManagerMessage> {
    ) -> mpsc::UnboundedSender<()> {
        let (tx, rx) = mpsc::unbounded_channel();

        let manager: RootManager = RootManager {
            channel_id,
            // broadcast_id: None,
            // detail_sender,
            // //
            // sender_by_connection_id: HashMap::new(),
            // new_disconnect_futures: Vec::new(),
            // //
            // tracker_connection_id: None,
        };

        let _ = tokio::spawn(manager.main(rx));
        tx
    }

    // async fn main(mut self, receiver: mpsc::UnboundedReceiver<RootManagerMessage>) {
    async fn main(mut self, receiver: mpsc::UnboundedReceiver<()>) {
        info!(id=?self.channel_id, "START CHANNEL MANAGER");
        let (mut status_sender, status_reciever) = mpsc::unbounded_channel::<ChannelControl>();
        // info!(channel_id = ?&self.channel_id,"START RootManager");

        loop {
            break;
        }
        info!(id=?self.channel_id, "STOP CHANNEL MANAGER");
    }
}
