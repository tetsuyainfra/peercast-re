use std::sync::Arc;

use hyper_util::server::graceful;
use libpeercast_re::{
    pcp::{ChannelInfo, GnuId, TrackInfo, connection::PcpConnection},
    repository::{Channel, shared_repository::SharedRepository},
};
use tokio::sync::watch;

use crate::channel::json_model::JsonChannel;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct RootConfig2 {}
pub type RootRepository2 = SharedRepository<RootChannel2>;

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
    type Config = RootConfig2;

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

    // fn before_delete(&mut self) {}

    fn id(&self) -> GnuId {
        self.0.id()
    }

    // fn state(&self) -> libpeercast_re::repository::ChannelState{ std::unimplemented!() }

    // fn channel_type(&self) -> libpeercast_re::repository::ChannelType { std::unimplemented!() }

    fn config(&self) -> Option<&Self::Config> {
        self.0.config()
    }

    fn channel_info(&self) -> Option<ChannelInfo> {
        self.0.channel_info()
    }
    fn track_info(&self) -> Option<TrackInfo> {
        self.0.track_info()
    }

    // fn created_at(&self) -> chrono::DateTime<chrono::Utc> { std::unimplemented!() }

    // fn updated_at(&self) -> chrono::DateTime<chrono::Utc> { std::unimplemented!() }

    // fn viewed_at(&self) -> chrono::DateTime<chrono::Utc> { std::unimplemented!() }
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
    // pub fn attache_connection(
    //     &self,
    //     conn: PcpConnection,
    //     graceful_shutdown: tokio_util::sync::CancellationToken,
    //     closed_send: tokio::sync::watch::Receiver<()>,
    // ) {
    //     todo!()
    // }
}

impl From<&RootChannel2> for JsonChannel {
    fn from(ch: &RootChannel2) -> Self {
        JsonChannel {
            id: todo!(),
            name: todo!(),
            tracker_addr: todo!(),
            contact_url: todo!(),
            genre: todo!(),
            raw_genre: todo!(),
            desc: todo!(),
            comment: todo!(),
            stream_type: todo!(),
            stream_ext: todo!(),
            bitrate: todo!(),
            number_of_listener: todo!(),
            number_of_relay: todo!(),
            created_at: todo!(),
            track: todo!(),
            typee: todo!(),
        }
    }
}

pub struct AttachTaskFuture;

impl Future for AttachTaskFuture {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        std::task::Poll::Ready(())
    }

}


#[derive(Debug)]
struct ImplChannel {
    id: libpeercast_re::pcp::GnuId,
    channel_info: watch::Receiver<Option<ChannelInfo>>,
    track_info: watch::Receiver<Option<TrackInfo>>,
    config: RootConfig2,
}

impl ImplChannel {
    fn new(
        id: libpeercast_re::pcp::GnuId,
        channel_info: Option<ChannelInfo>,
        track_info: Option<TrackInfo>,
        config: Option<RootConfig2>,
    ) -> Self {
        let mut config = config.unwrap_or_default();
        let (_tx, channel_info) = watch::channel(channel_info);
        let (_tx, track_info) = watch::channel(track_info);

        Self {
            config,
            id,
            channel_info,
            track_info,
        }
    }

    fn after_create(&self) -> impl Future<Output = ()> + Send {
        async {
            let _ = tokio::spawn(async {});
        }
    }

    // fn before_delete(&mut self) {}

    fn id(&self) -> GnuId {
        self.id
    }

    // fn state(&self) -> libpeercast_re::repository::ChannelState{ std::unimplemented!() }

    // fn channel_type(&self) -> libpeercast_re::repository::ChannelType { std::unimplemented!() }

    fn config(&self) -> Option<&RootConfig2> {
        Some(&self.config)
    }

    // fn update_config(&mut self, config: Self::Config) { std::unimplemented!() }

    fn channel_info(&self) -> Option<ChannelInfo> {
        self.channel_info.borrow().clone()
    }

    fn track_info(&self) -> Option<TrackInfo> {
        self.track_info.borrow().clone()
    }

    // fn created_at(&self) -> chrono::DateTime<chrono::Utc> { std::unimplemented!() }

    // fn updated_at(&self) -> chrono::DateTime<chrono::Utc> { std::unimplemented!() }

    // fn viewed_at(&self) -> chrono::DateTime<chrono::Utc> { std::unimplemented!() }
}
