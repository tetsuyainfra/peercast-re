use std::{
    net::{Shutdown, SocketAddr},
    sync::{Arc, Mutex},
};

use libpeercast_re::{
    pcp::{ChannelInfo, GnuId, TrackInfo, connection::PcpConnection},
    repository::{Channel, ChannelState, ChannelType},
    util::mutex_poisoned,
};
use tokio::sync::{
    mpsc::{self, UnboundedSender},
    watch,
};
use tokio_util::sync::CancellationToken;

use crate::prelude::*;
////////////////////////////////////////////////////////////////////////////////
// RootConfig
//
#[derive(Debug, Default, PartialEq, Eq)]
pub struct RootConfig {}

////////////////////////////////////////////////////////////////////////////////
//
//
#[derive(Debug)]
pub struct RootChannel2(Arc<ImplChannel>);

impl Clone for RootChannel2 {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl PartialEq for RootChannel2 {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for RootChannel2 {}

impl Channel for RootChannel2 {
    type Config = RootConfig;

    fn new(
        id: libpeercast_re::pcp::GnuId,
        channel_info: Option<ChannelInfo>,
        track_info: Option<TrackInfo>,
        config: Option<Self::Config>,
    ) -> Self {
        Self(Arc::new(ImplChannel::new(id, channel_info, track_info, config)))
    }

    fn after_create(&mut self) -> impl Future<Output = ()> + Send {
        self.0.after_create()
    }
    fn before_delete(&mut self) {
        self.0.before_delete()
    }

    fn cid(&self) -> GnuId {
        self.0.cid
    }
    fn state(&self) -> ChannelState {
        self.0.state()
    }
    fn channel_type(&self) -> libpeercast_re::repository::ChannelType {
        self.0.channel_type()
    }

    fn config(&self) -> Option<&Self::Config> {
        self.0.config()
    }
    // fn update_config(&mut self, config: Self::Config) { std::unimplemented!() }

    fn channel_info(&self) -> Option<ChannelInfo> {
        self.0.channel_info()
    }
    fn track_info(&self) -> Option<TrackInfo> {
        self.0.track_info()
    }

    fn created_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.0.created_at()
    }
    fn updated_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.0.updated_at()
    }
    fn viewed_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.0.viewed_at()
    }
}

impl RootChannel2 {
    pub fn attach_connection(
        self,
        pcp_connection: PcpConnection,
        graceful_shutdown: tokio_util::sync::CancellationToken,
        closed_send: watch::Receiver<()>,
    ) -> AttachTaskFuture {
        todo!()
    }
}

////////////////////////////////////////////////////////////////////////////////
//
//
pub struct AttachTaskFuture;

impl Future for AttachTaskFuture {
    type Output = ();

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        std::task::Poll::Ready(())
    }
}

////////////////////////////////////////////////////////////////////////////////
// ImplChannel
//
#[derive(Debug)]
struct ImplChannel {
    cid: libpeercast_re::pcp::GnuId,
    channel_info_rx: watch::Receiver<Option<ChannelInfo>>,
    track_info_rx: watch::Receiver<Option<TrackInfo>>,
    status_rx: watch::Receiver<ChannelState>,
    config: RootConfig,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at_rx: watch::Receiver<chrono::DateTime<chrono::Utc>>,
    viewed_at_rx: watch::Receiver<chrono::DateTime<chrono::Utc>>,

    ch_task_sender: UnboundedSender<ChMainMessage>,
}

impl ImplChannel {
    fn new(
        cid: libpeercast_re::pcp::GnuId,
        channel_info: Option<ChannelInfo>,
        track_info: Option<TrackInfo>,
        config: Option<RootConfig>,
    ) -> Self {
        let config = config.unwrap_or_default();

        let task_name = format!("Ch-{}", cid);
        let (ch_task_tx, ch_task_rx) = mpsc::unbounded_channel();
        let (channel_info_tx, channel_info_rx) = watch::channel(channel_info);
        let (track_info_tx, track_info_rx) = watch::channel(track_info);
        let (status_tx, status_rx) = watch::channel(ChannelState::Idle);
        let created_at = chrono::Utc::now();
        let (updated_at_tx, updated_at_rx) = watch::channel(created_at.clone());
        let (viewed_at_tx, viewed_at_rx) = watch::channel(created_at.clone());

        let worker = ChannelMainTask {
            task_name: task_name.clone(),
            cid,
            info_ch: InfoChannels {
                channel_info_tx,
                track_info_tx,
                status_tx,
                updated_at_tx,
                viewed_at_tx,
            },
            control_ch: ControlChannels {
                message: ch_task_rx,
            },
        };

        let _handle = tokio::task::Builder::new()
            .name(&task_name)
            .spawn(worker.run());

        Self {
            cid,
            channel_info_rx,
            track_info_rx,
            status_rx,
            config,
            created_at,
            updated_at_rx,
            viewed_at_rx,

            ch_task_sender: ch_task_tx,
        }
    }

    async fn after_create(&self) -> () {}
    fn before_delete(&self) -> () {
        let _ = self.ch_task_sender.send(ChMainMessage::ShutdownRequest);
    }

    fn state(&self) -> ChannelState {
        self.status_rx.borrow().clone()
    }

    fn channel_type(&self) -> ChannelType {
        ChannelType::Root
    }

    fn config(&self) -> Option<&RootConfig> {
        Some(&self.config)
    }

    // fn update_config(&mut self, config: Self::Config) { std::unimplemented!() }

    fn channel_info(&self) -> Option<ChannelInfo> {
        self.channel_info_rx.borrow().clone()
    }

    fn track_info(&self) -> Option<TrackInfo> {
        self.track_info_rx.borrow().clone()
    }

    fn created_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.created_at
    }
    fn updated_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.updated_at_rx.borrow().clone()
    }
    fn viewed_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.viewed_at_rx.borrow().clone()
    }
}
enum ChMainMessage {
    ConnectionAttached(),
    ConnectionClosed(),
    ShutdownRequest,
}

struct InfoChannels {
    channel_info_tx: watch::Sender<Option<ChannelInfo>>,
    track_info_tx: watch::Sender<Option<TrackInfo>>,
    status_tx: watch::Sender<ChannelState>,
    updated_at_tx: watch::Sender<chrono::DateTime<chrono::Utc>>,
    viewed_at_tx: watch::Sender<chrono::DateTime<chrono::Utc>>,
}

struct ControlChannels {
    message: mpsc::UnboundedReceiver<ChMainMessage>,
}

struct ChannelMainTask {
    task_name: String,
    cid: GnuId,
    info_ch: InfoChannels,
    control_ch: ControlChannels,
}

impl ChannelMainTask {
    pub async fn run(mut self) {
        info!("{}: Channel task started", self.task_name);
        loop {
            tokio::select! {
                Some(msg) = self.control_ch.message.recv() => {
                    match msg {
                        ChMainMessage::ConnectionAttached()=>{

                        },
                        ChMainMessage::ConnectionClosed()=>{
                        },
                        ChMainMessage::ShutdownRequest => {
                            info!("{}: Shutdown Request Recieved", self.task_name)
                        },
                    }
                }
                else => {
                    trace!("{}: Channel task receiver closed", self.task_name);
                    break;
                }
            }
        }
        info!("{}: Channel task ended", self.task_name);
    }

}
