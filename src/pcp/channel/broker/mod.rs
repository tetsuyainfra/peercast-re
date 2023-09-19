mod broker;

use bytes::Bytes;
use tokio::sync::mpsc;

use crate::{pcp::Atom, ConnectionId, rtmp::rtmp_connection::RtmpConnectionEvent, util::util_mpsc::send};

use super::{ChannelInfo, TrackInfo};


pub(crate) use broker::ChannelBroker;

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
    AtomHeadData {
        atom: Atom,
        payload: Bytes,
        pos: u32,
        info: Option<ChannelInfo>,
        track: Option<TrackInfo>,
    },
    AtomData {
        atom: Atom,
        data: Bytes,
        pos: u32,
        continuation: Option<bool>,
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
    AtomChanHead {
        atom: Atom,
        pos: u32,
        data: Bytes,
        // 送る必要ないのではないか
        // info: Option<ChannelInfo>,
        // track: Option<TrackInfo>,
    },
    AtomChanData {
        pos: u32,
        data: Bytes,
        can_be_dropped: bool,
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
        send(&mut broker_sender, message);

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
