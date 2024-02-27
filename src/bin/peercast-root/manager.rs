use std::{collections::HashMap, sync::Arc};

use futures_util::{
    future::{select_all, BoxFuture},
    FutureExt,
};
use peercast_re::{
    pcp::{decode::PcpBroadcast, GnuId},
    ConnectionId,
};
use tokio::sync::{
    mpsc::{self, UnboundedReceiver, UnboundedSender},
    watch,
};
use tracing::{debug, info, trace, warn};

use crate::channel::TrackerDetail;

//------------------------------------------------------------------------------
// RootManager
//
pub struct RootManager {
    channel_id: Arc<GnuId>,
    broadcast: Option<Arc<PcpBroadcast>>,
    pub detail_sender: watch::Sender<TrackerDetail>,
    // connection_idとSenderを組み合わせた物
    sender_by_connection_id: HashMap<ConnectionId, mpsc::UnboundedSender<ConnectionMessage>>,
    new_disconnect_futures: Vec<BoxFuture<'static, FutureResult>>,
}

impl std::fmt::Debug for RootManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RootManager")
            .field("channel_id", &self.channel_id)
            .field("broadcast", &self.broadcast)
            .field("sender_by_connection_id", &self.sender_by_connection_id)
            // .field("new_disconnect_futures", &self.new_disconnect_futures)
            .finish_non_exhaustive()
    }
}

impl RootManager {
    pub fn start(
        channel_id: Arc<GnuId>,
        detail_sender: watch::Sender<TrackerDetail>,
    ) -> mpsc::UnboundedSender<RootManagerMessage> {
        let (tx, rx) = mpsc::unbounded_channel();

        let manager: RootManager = RootManager {
            channel_id,
            broadcast: None,
            detail_sender,
            //
            sender_by_connection_id: HashMap::new(),
            new_disconnect_futures: Vec::new(),
        };

        let _ = tokio::spawn(manager.main(rx));
        tx
    }

    fn cleanup_connection(&mut self, connection_id: ConnectionId) {
        println!("REMOVE: RootManager is removing {}", connection_id);

        self.sender_by_connection_id.remove(&connection_id);
        //     if let Some(key) = self.key_by_connection_id.remove(&connection_id) {
        //         if let Some(players) = self.players_by_key.get_mut(&key) {
        //             players.remove(&connection_id);
        //         }

        //         if let Some(details) = self.publish_details.get_mut(&key) {
        //             if details.connection_id == connection_id {
        //                 self.publish_details.remove(&key);
        //             }
        //         }
        //     }
    }

    async fn main(mut self, receiver: UnboundedReceiver<RootManagerMessage>) {
        info!("START: RootManager {:?}", &self.channel_id);

        async fn new_receiver_future(
            mut receiver: UnboundedReceiver<RootManagerMessage>,
        ) -> FutureResult {
            let result = receiver.recv().await;
            FutureResult::MessageReceived {
                receiver,
                message: result,
            }
        }

        let mut futures = select_all(vec![new_receiver_future(receiver).boxed()]);

        'manager: loop {
            let (result, _index, remaining_futures) = futures.await;
            let mut new_futures = Vec::from(remaining_futures);

            match result {
                FutureResult::MessageReceived { receiver, message } => {
                    match message {
                        Some(message) => self.handle_message(message),
                        None => {
                            debug!("RootManagerMessage sender is all gone.");
                            break 'manager;
                        }
                    }

                    new_futures.push(new_receiver_future(receiver).boxed());
                }
                FutureResult::Disconnection { connection_id } => {
                    self.cleanup_connection(connection_id)
                }
            };

            for future in self.new_disconnect_futures.drain(..) {
                new_futures.push(future);
            }
            futures = select_all(new_futures);
        }

        info!("FINISH: RootManager {:?}", &self.channel_id);
    }

    fn handle_message(&mut self, message: RootManagerMessage) {
        match message {
            RootManagerMessage::NewConnection {
                connection_id,
                sender,
                disconnection,
            } => self.handle_new_connection(connection_id, sender, disconnection),
            RootManagerMessage::PublishChannel {
                session_id,
                broadcast_id,
                first_broadcast,
            } => self.handle_publish_channel(first_broadcast),
            RootManagerMessage::UpdateChannel { broadcast } => {
                self.handle_update_channel(broadcast)
            }
        }
    }

    // チャンネルに新規チャンネルが接続された
    fn handle_new_connection(
        &mut self,
        connection_id: ConnectionId,
        sender: UnboundedSender<ConnectionMessage>,
        disconnection: UnboundedReceiver<()>,
    ) {
        self.sender_by_connection_id.insert(connection_id, sender);
        self.new_disconnect_futures
            .push(wait_for_client_disconnection(connection_id, disconnection).boxed());
    }

    // チャンネルの配信開始
    fn handle_publish_channel(&mut self, frst_broadcast: Arc<PcpBroadcast>) {
        // if fistbroad cast arrived. we should check broadcast_id is same
    }

    // PcpBroadcastを元にチャンネル情報を更新する
    fn handle_update_channel(&mut self, broadcast: Arc<PcpBroadcast>) {
        let Some(group) = &broadcast.broadcast_group else {
            return;
        };
        match group.has_root() {
            true => (),
            false => return,
        };

        let _ = self.detail_sender.send(TrackerDetail {});
    }
}

enum FutureResult {
    Disconnection {
        connection_id: ConnectionId,
    },
    MessageReceived {
        receiver: UnboundedReceiver<RootManagerMessage>,
        message: Option<RootManagerMessage>,
    },
}

async fn wait_for_client_disconnection(
    connection_id: ConnectionId,
    mut receiver: UnboundedReceiver<()>,
) -> FutureResult {
    // The channel should only be closed when the client has disconnected
    while let Some(()) = receiver.recv().await {}

    FutureResult::Disconnection { connection_id }
}

/// Connection -> Manager メッセージ
#[derive(Debug)]
pub enum RootManagerMessage {
    NewConnection {
        connection_id: ConnectionId,
        sender: UnboundedSender<ConnectionMessage>,
        disconnection: UnboundedReceiver<()>,
    },

    PublishChannel {
        session_id: Arc<GnuId>,
        broadcast_id: Arc<GnuId>,
        first_broadcast: Arc<PcpBroadcast>,
    },
    UpdateChannel {
        broadcast: Arc<PcpBroadcast>,
    },
}

/// Manager -> Connection メッセージ
#[derive(Debug)]
pub enum ConnectionMessage {
    Ok {},
}
