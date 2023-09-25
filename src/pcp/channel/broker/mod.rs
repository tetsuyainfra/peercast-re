mod broadcast_broker;
mod relay_broker;

use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use bytes::Bytes;
use thiserror::Error;
use tokio::{sync::mpsc, task::JoinHandle};

use crate::{
    pcp::{Atom, GnuId},
    rtmp::rtmp_connection::RtmpConnectionEvent,
    util::util_mpsc::mpsc_send,
    ConnectionId,
};

use self::{broadcast_broker::BroadcastBrokerWoker, relay_broker::RelayBrokerWorker};

use super::{ChannelInfo, ChannelType, TrackInfo};

//------------------------------------------------------------------------------
// ChannelBroker Relation Struct
//

// BrokerWorkerが起こすエラー
#[derive(Debug, Error)]
enum BrokerError {}

/// マネージャへのメッセージ
#[derive(Debug)]
pub(crate) enum ChannelBrokerMessage {
    NewConnection {
        connection_id: ConnectionId,
        sender: mpsc::UnboundedSender<ChannelMessage>,
        disconnection: mpsc::UnboundedReceiver<()>,
    },
    UpdateChannelInfo {
        info: ChannelInfo,
        track: TrackInfo,
    },
    ArrivedChannelHead {
        atom: Atom,
        payload: Bytes,
        pos: u32,
        info: Option<ChannelInfo>,
        track: Option<TrackInfo>,
    },
    ArrivedChannelData {
        atom: Atom,
        payload: Bytes,
        pos: u32,
        continuation: bool,
    },
    AtomBroadcast {
        direction: AtomDirection,
        atom: Atom,
    },
    BroadcastEvent(RtmpConnectionEvent),
}

/// 各コネクションへのメッセージ
#[derive(Debug, Clone)]
pub enum ChannelMessage {
    RelayChannelHead {
        atom: Atom,
        pos: u32,
        payload: Bytes,
        info: Option<ChannelInfo>,
        track: Option<TrackInfo>,
    },
    RelayChannelData {
        atom: Atom,
        pos: u32,
        payload: Bytes,
        continuation: bool,
    },
    // AtomTrackerUpdate {
    //     info: Option<ChannelInfo>,
    //     track: Option<TrackInfo>,
    // },
    //
    // RtmpNewMetadata {
    //     metadata: StreamMetadata,
    // },
    // RtmpNewVideoData {
    //     timestamp: RtmpTimestamp,
    //     data: Bytes,
    //     can_be_dropped: bool,
    // },
    // RtmpNewAudioData {
    //     timestamp: RtmpTimestamp,
    //     data: Bytes,
    //     can_be_dropped: bool,
    // },
}

#[derive(Debug)]
pub enum AtomDirection {
    UpToDown, // Upstream To Downstream
    DownToUp,
}

//------------------------------------------------------------------------------
// ChannelBroker
//
#[derive(Debug)]
pub(crate) struct ChannelBroker {
    manager_tx: mpsc::UnboundedSender<ChannelBrokerMessage>,
    task: JoinHandle<Result<(), BrokerError>>,
    task_shutdown_tx: mpsc::UnboundedSender<()>,
}

impl ChannelBroker {
    pub fn new(
        channel_type: ChannelType,
        channel_id: GnuId,
        channel_info: Arc<RwLock<Option<ChannelInfo>>>,
        track_info: Arc<RwLock<Option<TrackInfo>>>,
    ) -> Self {
        let (manager_tx, manager_rx) = mpsc::unbounded_channel();
        let (task_shutdown_tx, task_shutdown_rx) = mpsc::unbounded_channel();

        let task = match &channel_type {
            ChannelType::Broadcast => {
                //
                // let broker: BroadcastBrokerWoker = ChannelBrokerWorker::new(
                //     channel_id,
                //     channel_info,
                //     track_info,
                //     task_shutdown_rx,
                // );
                // tokio::spawn(broker.start(manager_rx))
                let broker: BroadcastBrokerWoker = ChannelBrokerWorker::new(
                    channel_id,
                    channel_info,
                    track_info,
                    task_shutdown_rx,
                );
                tokio::spawn(broker.start(manager_rx))
            }
            ChannelType::Relay => {
                let broker: RelayBrokerWorker = ChannelBrokerWorker::new(
                    channel_id,
                    channel_info,
                    track_info,
                    task_shutdown_rx,
                );
                tokio::spawn(broker.start(manager_rx))
            }
        };

        Self {
            manager_tx,
            task,
            task_shutdown_tx,
        }
    }

    pub fn sender(&self) -> mpsc::UnboundedSender<ChannelBrokerMessage> {
        self.manager_tx.clone()
    }

    pub fn channel_reciever(&self, connection_id: ConnectionId) -> ChannelReciever {
        ChannelReciever::create(self.sender(), connection_id)
    }
}

//------------------------------------------------------------------------------
// ChannelBrokerWorker
//
#[async_trait]
trait ChannelBrokerWorker {
    fn new(
        channel_id: GnuId,
        channel_info: Arc<RwLock<Option<ChannelInfo>>>,
        track_info: Arc<RwLock<Option<TrackInfo>>>,
        shutdown_rx: mpsc::UnboundedReceiver<()>,
    ) -> Self;

    async fn start(
        mut self,
        manager_receiver: mpsc::UnboundedReceiver<ChannelBrokerMessage>,
    ) -> Result<(), BrokerError>;
}

//------------------------------------------------------------------------------
// ChannelBrokerReciever
//
#[derive(Debug)]
pub struct ChannelReciever {
    broker_sender: mpsc::UnboundedSender<ChannelBrokerMessage>,
    reciever_rx: mpsc::UnboundedReceiver<ChannelMessage>,
    disconnection_tx: mpsc::UnboundedSender<()>,
}
impl ChannelReciever {
    fn create(
        mut broker_sender: mpsc::UnboundedSender<ChannelBrokerMessage>,
        connection_id: ConnectionId,
    ) -> Self {
        let (reciever_tx, reciever_rx) = mpsc::unbounded_channel();
        let (disconnection_tx, disconnection) = mpsc::unbounded_channel();
        let message = ChannelBrokerMessage::NewConnection {
            connection_id: connection_id,
            sender: reciever_tx,
            disconnection: disconnection,
        };
        mpsc_send(&mut broker_sender, message);

        Self {
            broker_sender,
            reciever_rx,
            disconnection_tx,
        }
    }

    // ChanneMessageに二度目のが来る可能性が有るので気をつける用に
    pub async fn recv(&mut self) -> Option<ChannelMessage> {
        self.reciever_rx.recv().await
    }

    pub fn poll_recv(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<ChannelMessage>> {
        self.reciever_rx.poll_recv(cx)
    }
}
