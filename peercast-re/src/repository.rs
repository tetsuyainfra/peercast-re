#![allow(dead_code)]

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use chrono::{DateTime, Utc};
use libpeercast_re::{
    pcp::{ChannelInfo, GnuId, TrackInfo},
    util::mutex_poisoned,
};

////////////////////////////////////////////////////////////////////////////////
// Structs and Traits
//
pub trait Channel {
    type Config;

    fn new(
        self_session_id: GnuId,
        channel_id: GnuId,
        channel_info: Option<ChannelInfo>,
        track_info: Option<TrackInfo>,
        config: Option<Self::Config>,
    ) -> Self;

    // fn before_new(&mut self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
    fn before_new(&mut self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        Box::pin(async {})
    }

    fn last_update(&self) -> DateTime<Utc>;

    fn before_delete(&mut self) {}
}

pub struct ReChannelRepository<C> {
    session_id: GnuId,
    channels: Arc<Mutex<HashMap<GnuId, C>>>,
    delete_period_secs: u64,
    delete_check_interval_secs: u64,
}

enum DeleterMessage<C> {
    Delete(C),
    CheckExpire,
}

////////////////////////////////////////////////////////////////////////////////
// ReChannelRepository
//
impl<C> ReChannelRepository<C>
where
    C: Channel + Clone + Send + Sync + 'static + std::fmt::Debug,
{
    /// 削除期限
    const DELETE_PERIOD_SEC: u64 = 300;
    /// 削除チェックのインターバル時間
    const DELETE_CHECK_INTERVAL_SEC: u64 = 60;

    pub fn new(session_id: &GnuId) -> Self {
        let channels = Arc::new(Mutex::new(HashMap::default()));

        Self {
            session_id: session_id.clone(),
            channels,
            delete_period_secs: Self::DELETE_PERIOD_SEC,
            delete_check_interval_secs: Self::DELETE_CHECK_INTERVAL_SEC,
        }
    }

    pub fn new_with_ark(session_id: &GnuId) -> Arc<Self> {
        Arc::new(Self::new(session_id))
    }

    pub fn session_id(&self) -> GnuId {
        self.session_id.clone()
    }

    // HACKME: async 取り除きたい
    pub async fn create_or_get(
        &self,
        id: GnuId,
        channel_info: Option<ChannelInfo>,
        track_info: Option<TrackInfo>,
        config: Option<C::Config>,
    ) -> C {
        let (mut ch, is_init) = {
            // MutexGuardはSendを持っていないので、このブロック内でDropさせる
            let mut channels = self.channels.lock().unwrap_or_else(mutex_poisoned);
            match channels.get(&id) {
                Some(ch) => (ch.clone(), false),
                None => {
                    let ch = C::new(self.session_id.clone(), id.clone(), channel_info, track_info, config);
                    tracing::info!("Created new channel: {:?}", ch);
                    channels.insert(id.clone(), ch.clone());
                    (ch, true)
                }
            }
        };
        if is_init {
            ch.before_new().await;
        }
        ch
    }

    pub fn get(&self, id: &GnuId) -> Option<C> {
        let mut channels = self.channels.lock().unwrap_or_else(mutex_poisoned);
        match channels.get(id).clone() {
            Some(ch) => Some(ch.clone()),
            None => None,
        }
    }

    pub fn get_channels(&self) -> Vec<C> {
        let channels = self.channels.lock().unwrap_or_else(mutex_poisoned);
        channels.iter().map(|(_id, ch)| ch.clone()).collect()
    }

    pub fn delete(&self, id: &GnuId) -> bool {
        let mut channels = self.channels.lock().unwrap_or_else(mutex_poisoned);
        match channels.remove(&id) {
            Some(mut ch) => {
                ch.before_delete();
                drop(ch);
                true
            }
            None => false,
        }
    }

    // ------------------------------------------------------------------------------
    // Misc functions
    //
    pub fn channels_map(&self, func: fn(channels: &mut HashMap<GnuId, C>)) {
        let mut lock = self.channels.lock().unwrap();
        func(&mut (*lock));
    }

    pub fn map_collect<F, R>(&self, func: F) -> Vec<R>
    where
        F: FnMut((&GnuId, &C)) -> R,
    {
        let channels = self.channels.lock().unwrap_or_else(mutex_poisoned);
        channels.iter().map(func).collect()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_repository_delete_task() {
        // ReRepositoryDeleteTask::new()
    }
}

////////////////////////////////////////////////////////////////////////////////
// ReChannelRepositoryDeleteTask
//
/*
impl<C> ReChannelRepository<C>
where
    C: Channel + Clone + Send + 'static + std::fmt::Debug,
{
    pub fn create_deleter_task(&self) -> ReRepositoryDeleteTask<C> {
        let (deleter_sender, deleter_reciever) = unbounded_channel();
        self.deleter_sender.lock().unwrap_or_else(mutex_poisoned).replace(deleter_sender);

        ReRepositoryDeleteTask {
            channels: Arc::clone(&self.channels),
            delete_period_secs: self.delete_period_secs,
            delete_check_interval_secs: self.delete_check_interval_secs,
            deleter_reciever,
        }
    }
}
pub struct ReRepositoryDeleteTask<C> {
    channels: Arc<Mutex<HashMap<GnuId, C>>>,
    delete_period_secs: u64,
    delete_check_interval_secs: u64,
    deleter_reciever: UnboundedReceiver<DeleterMessage<C>>,
}

impl<C> ReRepositoryDeleteTask<C>
where
    C: Channel + Clone + Send + 'static + std::fmt::Debug,
{
    pub async fn start_delete_task(
        &self,
        shutdown_token: tokio_util::sync::CancellationToken,
        delete_period_secs: u64,
        delete_check_interval_secs: u64,
    ) {
        let (deleter_sender, deleter_reciever) = unbounded_channel();

        Self::deleter_task(
            delete_period_secs,
            delete_check_interval_secs,
            shutdown_token,
            Arc::clone(&self.channels),
            deleter_reciever,
        )
        .await
    }

    async fn deleter_task(
        delete_period_secs: u64,
        delete_check_interval_secs: u64,
        shutdown_token: tokio_util::sync::CancellationToken,
        channels: Arc<Mutex<HashMap<GnuId, C>>>,
        mut del_reciver: UnboundedReceiver<DeleterMessage<C>>,
    ) {
        let check_timeout_channel = || {
            let check_time = Utc::now() - Duration::from_secs(delete_period_secs); // before 5min
            let mut channels = channels.lock().unwrap_or_else(mutex_poisoned);
            let cid = channels
                .iter()
                .find(|c| c.1.last_update() < check_time)
                .map(|(cid, _)| cid.clone());
            if let Some(cid) = cid {
                channels.remove(&cid)
            } else {
                None
            }
        };

        let mut interval = tokio::time::interval(Duration::from_secs(delete_check_interval_secs));
        let mut check_interval = async || loop {
            interval.tick().await;
            match check_timeout_channel() {
                Some(ch) => return ch,
                None => continue,
            }
        };

        'main: loop {
            let mut channel = tokio::select! {
                _ = shutdown_token.cancelled() => {
                    tracing::info!("Shutting down ReChannelRepository deleter_task");
                    break 'main;
                }
                msg = del_reciver.recv() => {
                    match msg {
                        Some(msg) => {
                            match msg {
                                DeleterMessage::Delete(c) => c,
                                DeleterMessage::CheckExpire => {
                                    tracing::debug!("recived CheckExpire");
                                    match check_timeout_channel() {
                                        Some(ch) => ch,
                                        None => break 'main,
                                    }
                                },
                            }
                        },
                        None => break 'main,
                    }
                }
                ch = check_interval() => {
                    ch
                }
            };

            channel.before_delete();
            // 必ずしもここで破棄されるとは限らないことに注意
            // (MemberにArcが含まれていた場合、それを参照してるChannelが亡くなった時点でメンバーを含めてDropされる
            drop(channel)
        }

        tracing::info!("ReChannelRepository deleter_task exited");
    }
}
*/
