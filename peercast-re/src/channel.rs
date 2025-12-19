use crate::repository::Channel;



pub struct ReChannel {

}

pub struct ReConfig {

}


impl Channel for ReChannel {
    type Config = ReConfig;

    fn new(
        self_session_id: libpeercast_re::pcp::GnuId,
        id: libpeercast_re::pcp::GnuId,
        channel_info: Option<libpeercast_re::pcp::ChannelInfo>,
        track_info: Option<libpeercast_re::pcp::TrackInfo>,
        config: Option<Self::Config>,
    ) -> Self {
        todo!()
    }

    fn last_update(&self) -> chrono::DateTime<chrono::Utc> {
        todo!()
    }
}
