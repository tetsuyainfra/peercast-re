use std::borrow::BorrowMut;

use async_trait::async_trait;
use num::complex::ComplexFloat;
use rml_rtmp::sessions::StreamMetadata;
use thiserror::Error;
use tokio::{
    sync::{mpsc, watch},
    task::JoinHandle,
};
use tracing::{debug, error, info, trace};

use crate::{
    config,
    pcp::{channel::broker, Channel, ChannelInfo, GnuId, TrackInfo},
    rtmp::{
        rtmp_connection::{RtmpConnection, RtmpConnectionEvent},
        stream_manager::{ConnectionMessage, StreamManagerMessage},
    },
    util::util_mpsc::send,
    ConnectionId,
};
use broker::ChannelBrokerMessage;

use super::{SourceTaskConfig, SourceTaskTrait, TaskStatus};

#[derive(Debug, Clone)]
pub struct BroadcastTaskConfig {
    pub app_key: String,
    pub stream_key: String,
    pub rtmp_manager: mpsc::UnboundedSender<StreamManagerMessage>,
}

#[derive(Debug)]
enum WorkerStatus {
    Init,
    Idle,
    Recieving,
    Finished,
    Error,
}

#[derive(Debug)]
pub struct BroadcastTask {
    broker_sender: mpsc::UnboundedSender<ChannelBrokerMessage>,
    config: BroadcastTaskConfig,
    //
    worker_status: watch::Receiver<WorkerStatus>,
    worker: JoinHandle<Result<(), WorkerError>>,
}
impl From<BroadcastTaskConfig> for SourceTaskConfig {
    fn from(value: BroadcastTaskConfig) -> Self {
        SourceTaskConfig::Broadcast(value)
    }
}

impl BroadcastTask {
    pub(crate) fn new(
        channel_id: GnuId,
        connection_id: ConnectionId,
        config: BroadcastTaskConfig,
        broker_sender: mpsc::UnboundedSender<ChannelBrokerMessage>,
    ) -> Self {
        let (tx, rx) = watch::channel(WorkerStatus::Init);
        let config_clone = config.clone();
        let rtmp_manager_clone = config.rtmp_manager.clone();
        let broker_sender_clone = broker_sender.clone();

        let mut worker = BroadcastWorker::new(
            channel_id,
            connection_id,
            config_clone,
            rtmp_manager_clone,
            broker_sender_clone,
            tx,
        );

        let worker = tokio::spawn(async {
            let result = worker.start().await;
            result
        });

        Self {
            broker_sender,
            config,

            worker_status: rx,
            worker,
        }
    }
}

#[async_trait]
impl SourceTaskTrait for BroadcastTask {
    fn connect(&self, config: SourceTaskConfig) {}
    fn retry(&self) {}

    fn update_info(&self, info: ChannelInfo) {}
    fn update_track(&self, track: TrackInfo) {}

    fn status(&self) -> TaskStatus {
        match *self.worker_status.borrow() {
            WorkerStatus::Init => TaskStatus::Idle,
            WorkerStatus::Idle => TaskStatus::Idle,
            WorkerStatus::Recieving => TaskStatus::Running,
            WorkerStatus::Finished => TaskStatus::Stopped,
            WorkerStatus::Error => TaskStatus::Error,
        }
    }

    async fn status_changed(&mut self) -> Result<(), watch::error::RecvError> {
        todo!()
    }

    async fn stop(&self) {}
}

////////////////////////////////////////////////////////////////////////////////
// BroadcastWorker
//
#[derive(Debug, Error)]
enum WorkerError {
    #[error("error occured")]
    Message(String),
}

struct BroadcastWorker {
    channel_id: GnuId,
    connection_id: ConnectionId,
    config: BroadcastTaskConfig,
    //
    rtmp_manager: mpsc::UnboundedSender<StreamManagerMessage>,
    //
    broker_sender: mpsc::UnboundedSender<ChannelBrokerMessage>,
    //
    status_tx: watch::Sender<WorkerStatus>,
}

impl BroadcastWorker {
    fn new(
        channel_id: GnuId,
        connection_id: ConnectionId,
        config: BroadcastTaskConfig,
        // ワーカー内でリトライする必要は無いので直接RtmpConnectionを渡しても良い気がする
        rtmp_manager: mpsc::UnboundedSender<StreamManagerMessage>,
        //
        broker_sender: mpsc::UnboundedSender<ChannelBrokerMessage>,
        status_tx: watch::Sender<WorkerStatus>,
    ) -> Self {
        Self {
            channel_id,
            connection_id,
            config,
            rtmp_manager,
            broker_sender,
            status_tx,
        }
    }
    async fn start(mut self) -> Result<(), WorkerError> {
        info!("START BroadcastWorker CID:{}", &self.connection_id);
        //
        let (tx, mut rx) = mpsc::unbounded_channel();
        let (shutdown_tx, shutdown_rx) = mpsc::unbounded_channel();

        if !send(
            &mut self.broker_sender,
            ChannelBrokerMessage::NewConnection {
                connection_id: self.connection_id.clone(),
                sender: tx,
                disconnection: shutdown_rx,
            },
        ) {
            error!(
                " BroadcastWorker({}) ChannelBrokerMessage send failed",
                &self.connection_id
            );
            self.status_tx.send(WorkerStatus::Error);
            return Err(WorkerError::Message("cant send ChannelBroker".to_string()));
        };

        self.status_tx.send(WorkerStatus::Idle);

        let mut conn = RtmpConnection::new(
            self.rtmp_manager.clone(),
            self.connection_id,
            &self.config.app_key,
            &self.config.stream_key,
        );

        if !conn.connect().await {
            error!(
                " BroadcastWorker({}) RtmpConnection::connect() FAILED",
                &self.connection_id
            );
            self.status_tx.send(WorkerStatus::Error);
            return Err(WorkerError::Message(
                "cant connect RtmpConnection".to_string(),
            ));
        };
        info!(
            " BroadcastWorker({}) RtmpConnection::connect()",
            &self.connection_id
        );

        let mut results = vec![];

        info!(" BroadcastWorker({}) recieve start", &self.connection_id);
        let reason = loop {
            // Channel Brokerへ送るメッセージ
            let reaction = self
                .handle_session_results(&mut results)
                .map_err(|x| WorkerError::Message(format!("error")))?;
            if reaction == BroadcastConnectionReaction::Disconnect {
                info!("ConnectionReaction::Disconnect");
                break WorkerStatus::Finished;
            }

            tokio::select! {
                // RtmpConnectionから受け取るデータ
                data = conn.recv() => {
                    match data {
                        Some(msg) => {
                            // debug!(?msg);
                            let result = BroadcastSessionResult::RaisedEvent(msg);
                            results.push(result);
                        },
                        None => break WorkerStatus::Idle,
                    }
                }
                // Channel Brokerからのメッセージ
                msg = rx.recv() => {}
            }
        };

        drop(shutdown_tx);
        self.status_tx.send(reason);

        debug!("SHUTDOWN BroadcastTask CID:{}", &self.channel_id);

        Ok(())
    }

    fn handle_session_results(
        &mut self,
        results: &mut Vec<BroadcastSessionResult>,
    ) -> Result<BroadcastConnectionReaction, Box<dyn std::error::Error + Sync + Send>> {
        if results.len() == 0 {
            return Ok(BroadcastConnectionReaction::None);
        }

        let mut new_results = Vec::new();
        for result in results.drain(..) {
            let message = match result {
                // リモートへ送るのは無い。何故なら内部接続なので
                // SessionResult::OutboundResponse(a) => {
                //     //
                // }

                // イベントが発生した場合
                BroadcastSessionResult::RaisedEvent(event) => {
                    // trace!("RaisedEvent here, {:#?}", &event);
                    let action = self.handle_raised_event(event)?;
                    if action == BroadcastConnectionReaction::Disconnect {
                        return Ok(BroadcastConnectionReaction::Disconnect);
                    }
                }
                BroadcastSessionResult::Unknown => {} // SessionResult::Unknown(a) => {
                                                      //     warn!("unknown atom arrived {:?}[{}]", a.id(), a.len());
                                                      // }
            };
        }
        self.handle_session_results(&mut new_results);

        Ok(BroadcastConnectionReaction::None)
    }

    fn handle_raised_event(
        &mut self,
        event: RtmpConnectionEvent,
    ) -> Result<BroadcastConnectionReaction, Box<dyn std::error::Error + Sync + Send>> {
        trace!(handle_raised_event=?event);
        if !send(
            &self.broker_sender,
            ChannelBrokerMessage::BroadcastEvent(event),
        ) {
            return Ok(BroadcastConnectionReaction::Disconnect);
        }
        Ok(BroadcastConnectionReaction::None)
    }
}

enum BroadcastSessionResult {
    RaisedEvent(RtmpConnectionEvent),
    Unknown,
}

#[derive(Debug, PartialEq)]
enum BroadcastConnectionReaction {
    None,
    Disconnect,
}
