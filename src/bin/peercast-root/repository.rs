use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use chrono::{DateTime, Utc};
use peercast_re::{
    pcp::{ChannelInfo, GnuId, TrackInfo},
    util::mutex_poisoned,
};
use tokio::{
    io::DuplexStream,
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
    time::{interval, Interval},
};
use tracing::{debug, info};

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

pub struct ChannelRepository<C> {
    session_id: GnuId,
    channels: Arc<Mutex<HashMap<GnuId, C>>>,
    delete_period_secs: u64,
    delete_check_interval_secs: u64,
    deleter_sender: UnboundedSender<DeleterMessage<C>>,
    deleter_task: Arc<JoinHandle<()>>,
}

impl<C> ChannelRepository<C>
where
    C: Channel + Clone + Send + 'static + std::fmt::Debug,
{
    /// 削除期限
    const DELETE_PERIOD_SEC: u64 = 300;
    /// 削除チェックのインターバル時間
    const DELETE_CHECK_INTERVAL_SEC: u64 = 60;

    pub fn new(session_id: &GnuId) -> Self {
        let channels = Arc::new(Mutex::new(HashMap::default()));
        let (deleter_sender, deleter_reciever) = unbounded_channel();
        let deleter_task =
            tokio::spawn(Self::deleter_task(Arc::clone(&channels), deleter_reciever));

        Self {
            session_id: session_id.clone(),
            channels,
            deleter_sender,
            deleter_task: deleter_task.into(),
            delete_period_secs: Self::DELETE_PERIOD_SEC,
            delete_check_interval_secs: Self::DELETE_CHECK_INTERVAL_SEC,
        }
    }

    async fn deleter_task(
        channels: Arc<Mutex<HashMap<GnuId, C>>>,
        mut del_reciver: UnboundedReceiver<DeleterMessage<C>>,
    ) {
        let check_timeout_channel = || {
            let check_time = Utc::now() - Duration::from_secs(Self::DELETE_PERIOD_SEC); // before 5min
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

        let mut interval =
            tokio::time::interval(Duration::from_secs(Self::DELETE_CHECK_INTERVAL_SEC));
        let mut check_interval = async || loop {
            interval.tick().await;
            match check_timeout_channel() {
                Some(ch) => return ch,
                None => continue,
            }
        };

        'main: loop {
            let mut channel = tokio::select! {
                msg = del_reciver.recv() => {
                    match msg {
                        Some(msg) => {
                            match msg {
                                DeleterMessage::Delete(c) => c,
                                DeleterMessage::CheckExpire => {
                                    debug!("recived CheckExpire");
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
    }

    pub fn new_with_ark(session_id: &GnuId) -> Arc<Self> {
        Arc::new(Self::new(session_id))
    }

    pub fn session_id(&self) -> GnuId {
        self.session_id.clone()
    }

    pub fn channels_lock(&self, func: fn(channels: &mut HashMap<GnuId, C>)) {
        let mut lock = self.channels.lock().unwrap();
        func(&mut (*lock));
    }

    pub fn create_or_get(
        &self,
        id: GnuId,
        channel_info: Option<ChannelInfo>,
        track_info: Option<TrackInfo>,
        config: Option<C::Config>,
    ) -> C {
        let mut channels = match self.channels.lock() {
            Ok(c) => c,
            Err(_) => todo!(),
        };
        match channels.get(&id) {
            Some(ch) => ch.clone(),
            None => {
                // channelが無かった場合
                let channel = Channel::new(self.session_id, id, channel_info, track_info, config);
                match channels.insert(id, channel) {
                    Some(id) => panic!("ChannelManager have same GnuID. {:?}", &self.channels),
                    None => {
                        let ch = channels.get(&id).unwrap().clone();
                        info!("created channels. {:?}", &ch);
                        ch
                    }
                }
            }
        }
    }

    pub fn delete(&self, id: &GnuId) -> bool {
        let mut channels = self.channels.lock().unwrap_or_else(mutex_poisoned);
        match channels.remove(&id) {
            Some(ch) => self.deleter_sender.send(DeleterMessage::Delete(ch)).is_ok(),
            None => false,
        }
    }

    pub fn check_expire(&self) {
        self.deleter_sender.send(DeleterMessage::CheckExpire);
    }

    pub fn get(&self, id: &GnuId) -> Option<C> {
        let channels = match self.channels.lock() {
            Ok(c) => c,
            Err(_) => todo!(),
        };
        match channels.get(id).clone() {
            Some(ch) => Some(ch.clone()),
            None => None,
        }
    }

    pub fn get_channels(&self) -> Vec<C> {
        let channels = self.channels.lock().unwrap_or_else(mutex_poisoned);
        channels.iter().map(|(_id, ch)| ch.clone()).collect()
    }

    pub fn map_collect<F, R>(&self, func: F) -> Vec<R>
    where
        F: FnMut((&GnuId, &C)) -> R,
    {
        let channels = self.channels.lock().unwrap_or_else(mutex_poisoned);
        channels.iter().map(func).collect()
    }
}

enum DeleterMessage<C> {
    Delete(C),
    CheckExpire,
}

#[cfg(test)]
mod t {
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };

    use chrono::{DateTime, Utc};
    use peercast_re::{
        pcp::{ChannelInfo, GnuId},
        util::mutex_poisoned,
    };
    use tokio::{
        sync::{
            mpsc::{unbounded_channel, UnboundedSender},
            watch,
        },
        time::sleep,
    };
    use tracing::info;

    use crate::test_helper;

    use super::{Channel, ChannelRepository};

    #[derive(Debug, Clone)]
    struct TestChannel {
        cid: Arc<GnuId>,
        last_update: Arc<Mutex<DateTime<Utc>>>,
        //-- for test --
        sender: Arc<Mutex<Option<UnboundedSender<()>>>>,
    }

    impl Channel for TestChannel {
        type Config = ();
        fn new(
            session_id: peercast_re::pcp::GnuId,
            id: peercast_re::pcp::GnuId,
            channel_info: Option<peercast_re::pcp::ChannelInfo>,
            track_info: Option<peercast_re::pcp::TrackInfo>,
            option: Option<()>,
        ) -> Self {
            let last_update = Mutex::new(Utc::now()).into();
            TestChannel {
                cid: id.into(),
                last_update,
                sender: Mutex::new(None).into(),
            }
        }

        fn last_update(&self) -> DateTime<Utc> {
            self.last_update
                .lock()
                .unwrap_or_else(mutex_poisoned)
                .clone()
        }
    }

    #[tokio::test]
    async fn test_repository() {
        let self_session_id = GnuId::new();
        let mut repo = ChannelRepository::<TestChannel>::new_with_ark(&self_session_id);

        let cid = GnuId::new();
        let ch1 = repo.create_or_get(cid.clone(), None, None, None);
        let ch1_a = repo.create_or_get(cid.clone(), None, None, None);
        let ch1_b = repo.get(&cid).unwrap();
        assert_eq!(ch1.cid, ch1_a.cid);
        assert_eq!(ch1.cid, ch1_b.cid);
        let new_now = Utc::now();
        {
            *ch1.last_update.lock().unwrap_or_else(mutex_poisoned) = new_now.clone();
        }
        assert_eq!(ch1.last_update(), ch1_a.last_update());
        assert_eq!(ch1.last_update(), ch1_b.last_update());

        let r = repo.delete(&cid);
        assert!(r);

        let r = repo.delete(&GnuId::new());
        assert!(!r);
    }

    #[tokio::test]
    async fn test_delete_channel() {
        // test_helper::init_logger("debug");
        let self_session_id = GnuId::new();
        let mut repo = ChannelRepository::<TestChannel>::new_with_ark(&self_session_id);

        let cid = GnuId::new();
        let ch = repo.create_or_get(cid.clone(), None, None, None);
        assert!(repo.get(&cid).is_some());
        repo.delete(&cid);
        assert!(repo.get(&cid).is_none());

        let (sender, mut reciever) = unbounded_channel();
        let cid = GnuId::new();
        {
            let ch = repo.create_or_get(cid.clone(), None, None, None);
            {
                *ch.sender.lock().unwrap_or_else(mutex_poisoned) = Some(sender);
            }
            {
                // 最後のアップデートが300秒前とする
                *ch.last_update.lock().unwrap_or_else(mutex_poisoned) = Utc::now()
                    - Duration::from_secs(ChannelRepository::<TestChannel>::DELETE_PERIOD_SEC);
            }
            repo.check_expire();
            // Channelはすべて破棄されている
        }
        // senderが閉じている = channelが無くなったということ
        assert_eq!(reciever.recv().await, None);
        // 無くなっている
        assert!(repo.get(&cid).is_none());
    }
}
