use std::sync::atomic::{AtomicU32, Ordering};

use bytes::Bytes;
use rml_rtmp::{messages::RtmpMessage, sessions::StreamMetadata, time::RtmpTimestamp};
use tokio::sync::mpsc;
use tracing::debug;

use crate::{rtmp::send, ConnectionId};

use super::stream_manager::{ConnectionMessage, StreamManagerMessage};

pub struct RtmpConnection {
    manager_sender: mpsc::UnboundedSender<StreamManagerMessage>,
    connection_id: ConnectionId,
    rtmp_app: String,
    stream_key: String,
    //
    reciever: Option<mpsc::UnboundedReceiver<ConnectionMessage>>,
    _shutdown: Option<mpsc::UnboundedSender<()>>,
    //
    request_id_counter: AtomicU32,
}

impl RtmpConnection {
    pub fn new(
        manager_sender: mpsc::UnboundedSender<StreamManagerMessage>,
        connection_id: ConnectionId,
        rtmp_app: &str,
        stream_key: &str,
    ) -> Self {
        Self {
            manager_sender,
            connection_id,
            rtmp_app: rtmp_app.into(),
            stream_key: stream_key.into(),
            //
            reciever: None,
            _shutdown: None,
            //
            request_id_counter: AtomicU32::new(0),
        }
    }

    pub async fn connect(&mut self) -> bool {
        debug!("connect");
        let (tx, mut rx) = mpsc::unbounded_channel();
        let (shutdown_tx, shutdown_tr) = mpsc::unbounded_channel();
        let msg = StreamManagerMessage::NewConnection {
            connection_id: self.connection_id.0,
            sender: tx,
            disconnection: shutdown_tr,
        };
        if !send(&mut self.manager_sender, msg) {
            return false;
        }

        let req_id = self.request_id_counter.fetch_add(1, Ordering::SeqCst);
        let msg = StreamManagerMessage::PlaybackRequest {
            connection_id: self.connection_id.0,
            rtmp_app: self.rtmp_app.clone(),
            stream_key: self.stream_key.clone(),
            request_id: req_id,
        };
        if !send(&mut self.manager_sender, msg) {
            return false;
        }
        // debug!(?msg);
        let Some(msg) = rx.recv().await else {
            return false;
        };
        debug!(?msg);
        match msg {
            ConnectionMessage::RequestAccepted { request_id } => {
                assert_eq!(req_id, request_id);
            }
            _ => return false,
        }

        self.reciever = Some(rx);
        self._shutdown = Some(shutdown_tx);

        true
    }

    pub async fn recv(&mut self) -> Option<RtmpConnectionEvent> {
        if self.reciever.is_none() {
            return None;
        }
        self.reciever
            .as_mut()
            .unwrap()
            .recv()
            .await
            .map(Self::connection_message_to_rtmp_message)
    }

    fn connection_message_to_rtmp_message(
        connection_message: ConnectionMessage,
    ) -> RtmpConnectionEvent {
        match connection_message {
            ConnectionMessage::RequestAccepted { request_id } => {
                unimplemented!()
            }
            ConnectionMessage::RequestDenied { request_id } => {
                unimplemented!()
            }
            ConnectionMessage::NewMetadata { metadata } => {
                RtmpConnectionEvent::NewMetadata { metadata }
            }
            ConnectionMessage::NewVideoData {
                timestamp,
                data,
                can_be_dropped,
            } => RtmpConnectionEvent::NewVideoData {
                timestamp,
                data,
                can_be_dropped,
            },
            ConnectionMessage::NewAudioData {
                timestamp,
                data,
                can_be_dropped,
            } => RtmpConnectionEvent::NewAudioData {
                timestamp,
                data,
                can_be_dropped,
            },
        }
    }
}

pub enum RtmpConnectionEvent {
    NewVideoData {
        timestamp: RtmpTimestamp,
        data: Bytes,
        can_be_dropped: bool,
    },
    NewAudioData {
        timestamp: RtmpTimestamp,
        data: Bytes,
        can_be_dropped: bool,
    },
    NewMetadata {
        metadata: StreamMetadata,
    },
}
impl std::fmt::Debug for RtmpConnectionEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NewVideoData {
                timestamp,
                data,
                can_be_dropped,
            } => f
                .debug_struct("NewVideoData")
                .field("timestamp", timestamp)
                // .field("data", data)
                .field("can_be_dropped", can_be_dropped)
                .finish_non_exhaustive(),
            Self::NewAudioData {
                timestamp,
                data,
                can_be_dropped,
            } => f
                .debug_struct("NewAudioData")
                .field("timestamp", timestamp)
                // .field("data", data)
                .field("can_be_dropped", can_be_dropped)
                .finish_non_exhaustive(),
            Self::NewMetadata { metadata } => f
                .debug_struct("NewMetadata")
                .field("metadata", metadata)
                .finish(),
        }
    }
}
