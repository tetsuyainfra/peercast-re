use std::sync::{Arc, Mutex, RwLock};

use chrono::{DateTime, Utc};
use libpeercast_re::pcp::GnuId;

use crate::repository::Channel;

#[derive(Clone, Debug)]
pub struct ReChannel {
    cid: GnuId,
    channel_info: Arc<RwLock<libpeercast_re::pcp::ChannelInfo>>,
    track_info: Arc<RwLock<libpeercast_re::pcp::TrackInfo>>,
    number_of_listener: Arc<RwLock<i32>>,
    number_of_relay: Arc<RwLock<i32>>,
    last_update: Arc<Mutex<DateTime<Utc>>>,
    created_at: Arc<DateTime<Utc>>,
}

pub struct ReConfig {
}

impl Channel for ReChannel {
    type Config = ReConfig;

    fn new(
        self_session_id: GnuId,
        channel_id: GnuId,
        channel_info: Option<libpeercast_re::pcp::ChannelInfo>,
        track_info: Option<libpeercast_re::pcp::TrackInfo>,
        config: Option<Self::Config>,
    ) -> Self {
        let now_ = Utc::now();

        Self {
            cid: channel_id,
            channel_info: RwLock::new(channel_info.unwrap_or_default()).into(),
            track_info: RwLock::new(track_info.unwrap_or_default()).into(),
            number_of_listener: RwLock::new(0).into(),
            number_of_relay: RwLock::new(0).into(),
            last_update: Arc::new(Mutex::new(now_.clone())),
            created_at: Arc::new(now_),
        }
    }

    fn last_update(&self) -> chrono::DateTime<chrono::Utc> {
        todo!()
    }
}

impl ReChannel {
}
