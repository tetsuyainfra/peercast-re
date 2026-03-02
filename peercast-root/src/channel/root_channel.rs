use std::{net::SocketAddr, sync::Arc};

use libpeercast_re::{
    model::{ValidChannelInfo, ValidTrackInfo},
    pcp::{GnuId, connection::PcpConnection, connection5::ConnectionHandle},
    repository::{Channel, ChannelState, ChannelType},
};
use tokio::sync::{
    mpsc::{self, UnboundedSender},
    watch,
};

use crate::{
    connection::{RootHandle, RootSpec},
    prelude::*,
};
////////////////////////////////////////////////////////////////////////////////
// RootConfig
//
#[derive(Debug, Default, PartialEq, Eq)]
pub struct RootConfig {
    pub broadcast_id: GnuId,
    pub tracker_addr: Option<SocketAddr>,
}

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
        channel_info: Option<ValidChannelInfo>,
        track_info: Option<ValidTrackInfo>,
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
    fn update_config(&mut self, _config: Self::Config) {
        todo!()
    }

    fn tracker_address(&self) -> Option<std::net::SocketAddr> {
        self.0.tracker_address()
    }
    fn channel_info(&self) -> Option<ValidChannelInfo> {
        self.0.channel_info()
    }
    fn track_info(&self) -> Option<ValidTrackInfo> {
        self.0.track_info()
    }
    fn number_of_listener(&self) -> i32 {
        self.0.number_of_listener()
    }
    fn number_of_relay(&self) -> i32 {
        self.0.number_of_relay()
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
    pub fn control(&self) -> ChannelControl {
        ChannelControl {}
    }
    pub fn control_with_authenticate(&self, _broadcast_id: GnuId) -> AuthedChannelControl {
        AuthedChannelControl {}
    }

    pub fn attach_connection(&self, root_connection_handle: RootHandle) {
        self.0.attach_connection(root_connection_handle);
    }
}

////////////////////////////////////////////////////////////////////////////////
// ChannelControl, AuthedChannelControl
//
#[derive(Debug)]
pub struct ChannelControl {}

#[derive(Debug)]
pub struct AuthedChannelControl {}
////////////////////////////////////////////////////////////////////////////////
//
//
pub struct AttachTaskFuture;

impl Future for AttachTaskFuture {
    type Output = ();

    fn poll(self: std::pin::Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        std::task::Poll::Ready(())
    }
}

////////////////////////////////////////////////////////////////////////////////
// ImplChannel
//
#[derive(Debug)]
struct ImplChannel {
    cid: libpeercast_re::pcp::GnuId,
    config: RootConfig,
    //
    ch_task_sender: UnboundedSender<ChMainMessage>,
    //
    tracker_addr_rx: watch::Receiver<Option<SocketAddr>>,
    channel_info_rx: watch::Receiver<Option<ValidChannelInfo>>,
    track_info_rx: watch::Receiver<Option<ValidTrackInfo>>,
    status_rx: watch::Receiver<ChannelState>,
    number_of_listener_rx: watch::Receiver<i32>,
    number_of_relay_rx: watch::Receiver<i32>,
    //
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at_rx: watch::Receiver<chrono::DateTime<chrono::Utc>>,
    viewed_at_rx: watch::Receiver<chrono::DateTime<chrono::Utc>>,
}

impl ImplChannel {
    fn new(
        cid: libpeercast_re::pcp::GnuId,
        channel_info: Option<ValidChannelInfo>,
        track_info: Option<ValidTrackInfo>,
        config: Option<RootConfig>,
    ) -> Self {
        let config = config.unwrap_or_default();

        let task_name = format!("Ch-{}", cid);
        let (ch_task_tx, ch_task_rx) = mpsc::unbounded_channel();
        let (tracker_addr_tx, tracker_addr_rx) = watch::channel(config.tracker_addr);
        let (channel_info_tx, channel_info_rx) = watch::channel(channel_info);
        let (track_info_tx, track_info_rx) = watch::channel(track_info);
        let (status_tx, status_rx) = watch::channel(ChannelState::Idle);
        let (number_of_listener_tx, number_of_listener_rx) = watch::channel(0);
        let (number_of_relay_tx, number_of_relay_rx) = watch::channel(0);
        let created_at = chrono::Utc::now();
        let (updated_at_tx, updated_at_rx) = watch::channel(created_at.clone());
        let (viewed_at_tx, viewed_at_rx) = watch::channel(created_at.clone());

        let worker = ChannelMainTask {
            task_name: task_name.clone(),
            cid,
            info_ch: InfoChannels {
                tracker_addr_tx,
                channel_info_tx,
                track_info_tx,
                status_tx,
                number_of_listener_tx,
                number_of_relay_tx,
                updated_at_tx,
                viewed_at_tx,
            },
            control_ch: ControlChannels {
                message: ch_task_rx,
            },
            //
            connection_handle: None,
        };

        let _handle = tokio::task::Builder::new().name(&task_name).spawn(worker.run());

        Self {
            cid,
            config,
            //
            tracker_addr_rx,
            channel_info_rx,
            track_info_rx,
            status_rx,
            number_of_listener_rx,
            number_of_relay_rx,
            //
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

    fn tracker_address(&self) -> Option<std::net::SocketAddr> {
        self.tracker_addr_rx.borrow().clone()
    }
    fn channel_info(&self) -> Option<ValidChannelInfo> {
        self.channel_info_rx.borrow().clone()
    }
    fn track_info(&self) -> Option<ValidTrackInfo> {
        self.track_info_rx.borrow().clone()
    }
    fn number_of_listener(&self) -> i32 {
        self.number_of_listener_rx.borrow().clone()
    }
    fn number_of_relay(&self) -> i32 {
        self.number_of_relay_rx.borrow().clone()
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

    pub fn attach_connection(&self, root_connection_handle: RootHandle) {
        let _ = self.ch_task_sender.send(ChMainMessage::ConnectionAttached(root_connection_handle));
    }
}

#[allow(dead_code)]
enum ChMainMessage {
    ConnectionAttached(RootHandle),
    ConnectionClosed(),
    ShutdownRequest,
}

#[allow(dead_code)]
struct InfoChannels {
    tracker_addr_tx: watch::Sender<Option<SocketAddr>>,
    channel_info_tx: watch::Sender<Option<ValidChannelInfo>>,
    track_info_tx: watch::Sender<Option<ValidTrackInfo>>,
    status_tx: watch::Sender<ChannelState>,
    //
    number_of_listener_tx: watch::Sender<i32>,
    number_of_relay_tx: watch::Sender<i32>,

    //
    updated_at_tx: watch::Sender<chrono::DateTime<chrono::Utc>>,
    viewed_at_tx: watch::Sender<chrono::DateTime<chrono::Utc>>,
    //
}

struct ControlChannels {
    message: mpsc::UnboundedReceiver<ChMainMessage>,
}

#[allow(dead_code)]
struct ChannelMainTask {
    task_name: String,
    cid: GnuId,
    info_ch: InfoChannels,
    control_ch: ControlChannels,
    //
    connection_handle: Option<RootHandle>,
}

impl ChannelMainTask {
    pub async fn run(mut self) {
        info!("{}: Channel task started", self.task_name);
        loop {
            tokio::select! {
                Some(msg) = self.control_ch.message.recv() => {
                    match msg {
                        ChMainMessage::ConnectionAttached(connection_handle)=>{
                            if let Some(old_connection) = self.connection_handle.take() {
                                old_connection.shutdown();
                            }
                            self.connection_handle = Some(connection_handle)
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
