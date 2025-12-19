#![allow(dead_code)]

use std::{collections::HashMap, sync::{Arc, Mutex}};

use chrono::{DateTime, Utc};
use libpeercast_re::pcp::{ChannelInfo, GnuId, TrackInfo};
use tokio::{sync::mpsc::UnboundedSender, task::JoinHandle};

pub trait Channel {
    type Config;
    fn new(
        self_session_id: GnuId,
        id: GnuId,
        channel_info: Option<ChannelInfo>,
        track_info: Option<TrackInfo>,
        config: Option<Self::Config>,
    ) -> Self;

    fn last_update(&self) -> DateTime<Utc>;

    fn before_delete(&mut self) {}
}

pub struct ReChannelRepository<C> {
    session_id: GnuId,
    channels: Arc<Mutex<HashMap<GnuId, C>>>,
    delete_period_secs: u64,
    delete_check_interval_secs: u64,
    deleter_sender: UnboundedSender<DeleterMessage<C>>,
    deleter_task: Arc<JoinHandle<()>>,
}

enum DeleterMessage<C> {
    Delete(C),
    CheckExpire,
}
