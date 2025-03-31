use std::{
    collections::VecDeque,
    net::SocketAddr,
    pin::Pin,
    sync::{Arc, RwLock},
    time::SystemTime,
};

use chrono::{DateTime, Utc};
use num::complex::ComplexFloat;
use tokio::{
    sync::{
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        watch,
    },
    task::JoinHandle,
};
use tracing::{debug, info, trace};

use crate::{
    pcp::{connection, GnuId},
    ConnectionId,
};

use super::{
    broker::ChannelBroker,
    channel_stream::ChannelStream,
    src_task::{BroadcastTask, RelayTask, SourceTask, SourceTaskConfig, TaskStatus},
    ChannelInfo, ChannelReciever, TrackInfo,
};

//------------------------------------------------------------------------------
// Relation Structs
//
#[derive(Debug, Clone)]
pub enum ChannelType {
    Broadcast, // { app: String, pass: String }, // 配信チャンネル(このPCで配信している)
    Relay,     //(SocketAddr),                       // 中継チャンネル
}

//------------------------------------------------------------------------------
// Channel
//
#[derive(Debug, Clone)]
pub struct Channel {
    session_id: GnuId,
    id: GnuId,
    ch_type: ChannelType, // 作成したら状態は変わらないで良いともうけども・・・Configは変わる可能性あるか？
    channel_info: Arc<RwLock<Option<ChannelInfo>>>,
    track_info: Arc<RwLock<Option<TrackInfo>>>,

    // manager
    broker_task: Arc<ChannelBroker>,
    // SourceTask
    // source_task: Arc<RwLock<Option<JoinHandle<()>>>>,
    // source_task: Arc<RwLock<Option<Pin<Box<dyn SourceTask>>>>>,
    source_task: Arc<RwLock<Option<Box<dyn SourceTask>>>>,
    //
    // shutdown: Arc<RwLock<Shutdown>>,
    //
    hosts: Arc<RwLock<VecDeque<SocketAddr>>>,
    hosts_tested: Arc<RwLock<Vec<SocketAddr>>>,

    //
    created_at: DateTime<Utc>,
}

impl Channel {
    pub(super) fn new(
        session_id: GnuId,
        id: GnuId,
        ch_type: ChannelType,
        channel_info: Option<ChannelInfo>,
        track_info: Option<TrackInfo>,
    ) -> Self {
        let channel_info = Arc::new(RwLock::new(channel_info));
        let track_info = Arc::new(RwLock::new(track_info));
        Channel {
            session_id,
            id,
            ch_type,
            //
            broker_task: Arc::new(ChannelBroker::new(
                ChannelType::Broadcast,
                id,
                Arc::clone(&channel_info),
                Arc::clone(&track_info),
            )),
            channel_info,
            track_info,
            // SourceTask for RtmpSorce / RelayFrom
            source_task: Arc::new(RwLock::new(None)),
            //
            // shutdown: Arc::new(RwLock::new(Shutdown::new())),
            //
            hosts: Default::default(),
            hosts_tested: Default::default(),

            //
            created_at: Utc::now(),
        }
    }

    pub fn id(&self) -> GnuId {
        self.id.clone()
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at.clone()
    }

    pub fn channel_type(&self) -> ChannelType {
        self.ch_type.clone()
    }

    pub fn info(&self) -> Option<ChannelInfo> {
        self.channel_info.read().unwrap().clone()
    }
    pub fn set_info(&self, info: ChannelInfo) {
        let mut lock = self.channel_info.write().unwrap();
        *lock = Some(info);
        // TOOD: send info to task
    }

    pub fn track(&self) -> Option<TrackInfo> {
        self.track_info.read().unwrap().clone()
    }
    pub fn set_track(&self, track: TrackInfo) {
        let mut lock = self.track_info.write().unwrap();
        *lock = Some(track);
        // TOOD: send info to task
    }

    pub fn connect(&self, connection_id: ConnectionId, config: SourceTaskConfig) -> bool {
        let mut opt_task = self.source_task.write().unwrap();
        let mut broker_sender = self.broker_task.sender();
        match opt_task.as_ref() {
            Some(c) => false,
            None => {
                // *opt_task = Some(pinned_task);
                *opt_task = match &config {
                    SourceTaskConfig::Broadcast(c) => {
                        //
                        let mut task = BroadcastTask::new(self.id(), connection_id, broker_sender);
                        let _ = task.connect(config);
                        // let task: Pin<Box<dyn SourceTask>> = Box::pin(task);
                        Some(Box::new(task))
                    }
                    SourceTaskConfig::Relay(c) => {
                        let mut task = RelayTask::new(self.session_id, self.id(), broker_sender);
                        let _ = task.connect(config);
                        // let task: Pin<Box<dyn SourceTask>> = Box::pin(task);
                        Some(Box::new(task))
                    }
                };
                true
            }
        }
    }

    pub fn stop(&self) {
        let mut opt_task = self.source_task.write().unwrap();
        match opt_task.take() {
            None => {}
            Some(task) => task.stop(),
        }
    }

    pub fn retry(&self) -> bool {
        let mut opt_task = self.source_task.write().unwrap();
        match opt_task.as_mut() {
            None => false,
            Some(task) => task.retry(),
        }
    }

    pub fn status(&self) -> TaskStatus {
        let mut opt_task = self.source_task.read().unwrap();
        match &(*opt_task) {
            Some(task) => task.status(),
            None => TaskStatus::Idle,
        }
    }

    pub fn channel_reciever(&self, connection_id: ConnectionId) -> ChannelReciever {
        self.broker_task.channel_reciever(connection_id)
    }

    pub fn channel_stream(&self, connection_id: ConnectionId) -> ChannelStream {
        let reciever = self.broker_task.channel_reciever(connection_id);
        ChannelStream::new(self.id.clone(), reciever)
    }
}

#[cfg(test)]
mod struct_future {
    use std::sync::{Arc, RwLock};

    use axum::extract::State;
    use futures_util::future::Join;
    use httparse::Status;
    use tokio::{sync::mpsc, task::JoinHandle};

    #[crate::test]
    async fn t() {
        struct Ch {
            task_state: Arc<RwLock<TaskState>>,
        }
        impl Ch {
            fn new() -> Self {
                Ch {
                    task_state: Arc::new(RwLock::new(TaskState::Idle)),
                }
            }
            async fn task_start(&mut self) -> bool {
                let mut state = self.task_state.write().unwrap();
                state.start().await
            }
            fn task_status(&self) -> Status {
                self.task_state.read().unwrap().status()
            }
        }
        enum Status {
            Idle,
            Running,
            Error,
            Finish,
        }
        enum TaskState {
            Idle,
            Running { handle: JoinHandle<()> },
            Error,
            Finish,
        }
        impl TaskState {
            async fn start(&mut self) -> bool {
                let handle = tokio::spawn(async {});
                *self = TaskState::Running { handle };
                true
            }
            fn status(&self) -> Status {
                match self {
                    TaskState::Idle => Status::Idle,
                    TaskState::Running { .. } => Status::Running,
                    TaskState::Error => Status::Error,
                    TaskState::Finish => Status::Finish,
                }
            }
        }

        let mut ch = Ch::new();
        ch.task_start().await;
        ch.task_start().await;
        // ch.wait().await
    }

    fn tt() {
        enum Status {
            Init,
            Running,
            Pause,
        }
        enum InStatus {
            Init,
            Running {},
            Pause,
        }

        impl InStatus {
            fn start(&mut self) -> bool {
                *self = match self {
                    InStatus::Running {} => return false,
                    InStatus::Init | InStatus::Pause => {
                        //
                        InStatus::Running {}
                    }
                };

                true
            }
            fn stop(&mut self) {
                *self = match self {
                    InStatus::Running {} | InStatus::Init | InStatus::Pause => InStatus::Pause {},
                };
            }
            fn status(&self) -> Status {
                match self {
                    InStatus::Init => Status::Init,
                    InStatus::Running { .. } => Status::Running,
                    InStatus::Pause => Status::Pause,
                }
            }
        }
    }
}
