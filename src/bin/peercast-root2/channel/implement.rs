#![allow(unused)]

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
    thread::JoinHandle,
};

use chrono::{DateTime, Utc};
use http::header::SEC_WEBSOCKET_ACCEPT;
use peercast_re::{
    pcp::GnuId,
    util::{mutex_poisoned, rwlock_read_poisoned, rwlock_write_poisoned},
};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{info, trace};

use super::{Channel, Repository};

//------------------------------------------------------------------------------
// Channel
//
#[derive(Debug, Clone)]
struct TrackerChannel {
    id: Arc<GnuId>,
    config: Arc<TrackerChannelConfig>,
    // detail_receiver: tokio::sync::watch::Receiver<ChannelDetail>,
    // watcher_sender_: tokio::sync::mpsc::UnboundedSender<ChannelWatcherMessage>,
    service: Arc<Mutex<ChannelService>>,
}

#[derive(Debug, Clone, PartialEq)]
struct TrackerChannelConfig {
    tracker_session_id: GnuId,
    tracker_broadcast_id: GnuId,
}

impl Channel for TrackerChannel {
    type Config = TrackerChannelConfig;

    fn new(
        channel_id: GnuId,
        config: Self::Config,
        // status_sender: watch::Sender<ChannelStatus>, // store: Weak<Self::InternalStore>,
        // watcher_sender: tokio::sync::mpsc::UnboundedSender<super::store::ChannelWatcherMessage>,
    ) -> Self {
        let service = ChannelService::new_shared(channel_id.clone());
        Self {
            id: channel_id.into(),
            config: config.into(),
            service,
            // detail_receiver: (),
        }
    }

    fn stop(&self) {
        self.service
            .lock()
            .unwrap_or_else(mutex_poisoned)
            .cancell_token
            .cancel();
    }

    fn id(&self) -> GnuId {
        self.id.as_ref().clone()
    }

    fn config(&self) -> Self::Config {
        self.config.as_ref().clone()
    }
}

impl PartialEq for TrackerChannel {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

//------------------------------------------------------------------------------
// ChannelService: Channelのタスク処理を行う構造体
//
#[derive(Debug)]
struct ChannelService {
    cancell_token: tokio_util::sync::CancellationToken,
    main_tx: tokio::sync::mpsc::UnboundedSender<ChannelServiceMessage>,
    handle: tokio::task::JoinHandle<()>,
    state_tr: tokio::sync::watch::Receiver<ChannelServiceState>,
}

impl ChannelService {
    fn new(id: GnuId) -> Self {
        let (main_tx, main_tr) = tokio::sync::mpsc::unbounded_channel();
        let cancell_token = tokio_util::sync::CancellationToken::new();
        let (state_tx, state_tr) = tokio::sync::watch::channel(ChannelServiceState::Initializing);
        let handle = tokio::spawn(Self::watcher_main(
            id,
            main_tr,
            cancell_token.clone(),
            state_tx,
        ));
        Self {
            cancell_token,
            main_tx,
            handle,
            state_tr,
        }
    }

    fn new_shared(id: GnuId) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::new(id)))
    }

    fn state(&mut self) -> ChannelServiceState {
        self.state_tr.borrow_and_update().clone()
    }

    async fn watcher_main(
        id: GnuId,
        mut tx: tokio::sync::mpsc::UnboundedReceiver<ChannelServiceMessage>,
        cancel_token: tokio_util::sync::CancellationToken,
        state_tr: tokio::sync::watch::Sender<ChannelServiceState>,
    ) {
        info!(?id, "START ChannelService");

        state_tr.send(ChannelServiceState::Start);
        'main: loop {
            tokio::select! {
                Some(msg) = tx.recv() => {
                    match msg {
                        // ChannelServiceMessage::RemoveChannel(channel_id) => {
                        //     match store.get(&channel_id) {
                        //         None => continue 'main,
                        //         Some(mut channel) => {
                        //             channel.before_remove(channel_id);
                        //         },
                        //     };
                        //     tokio::task::yield_now().await;
                        //     match store.remove(&channel_id) {
                        //         None => continue 'main,
                        //         Some(mut channel) => {
                        //             channel.after_remove(channel_id);
                        //         },
                        //     };
                        // },
                    }
                },
                _ = cancel_token.cancelled() => {
                    info!(?id, "ChannelService Cancelled");
                    break;
                },
                else => {
                    info!(?id, "ChannelService MSG SENDER IS ALL CLOSED");
                    break;
                }
            }
        }

        state_tr.send(ChannelServiceState::Finished);
        info!(?id, "STOP ChannelService");
    }
}

#[derive(Debug, Clone, PartialEq)]
enum ChannelServiceState {
    Initializing,
    Start,
    Finished,
}

#[derive(Debug, Clone)]
enum ChannelServiceMessage {}

//------------------------------------------------------------------------------
// ChannelDetail : Channelについての詳細な情報
//
#[derive(Debug, Clone)]
pub struct ChannelDetail {
    created_at: DateTime<Utc>,
}

impl ChannelDetail {
    fn new() -> Self {
        Self {
            created_at: DateTime::default(),
        }
    }
}

#[derive(Debug)]
struct InternalRepository {
    root_session_id_: GnuId,
    root_broadcast_id_: GnuId,
    channels_: HashMap<GnuId, TrackerChannel>,
    // repo_watcher
}

impl InternalRepository {
    fn new(
        root_session_id_: GnuId,
        root_broadcast_id_: GnuId,
        // watcher_sender_: mpsc::UnboundedSender<ChannelWatcherMessage>,
    ) -> Self {
        Self {
            channels_: Default::default(),
            root_session_id_,
            root_broadcast_id_,
        }
    }
    fn session_id(&self) -> GnuId {
        self.root_session_id_
    }

    fn broadcast_id(&self) -> GnuId {
        self.root_broadcast_id_
    }

    fn get(&self, channel_id: &GnuId) -> Option<TrackerChannel> {
        trace!(root_session_id=?self.root_session_id_, ?channel_id, "Get Channel");
        self.channels_
            // .unwrap_or_else(rwlock_read_poisoned)
            .get(channel_id)
            .map(|c| c.clone())
    }

    fn get_or_create(
        &mut self,
        channel_id: &GnuId,
        config: TrackerChannelConfig,
    ) -> TrackerChannel {
        info!(root_session_id=?self.root_session_id_, ?channel_id, "Create Channel");
        self.get(channel_id)
            .or_else(|| {
                // channelsの中に無いので新規作成する
                let ch = self.channels_.entry(channel_id.clone()).or_insert_with(
                    || -> TrackerChannel { TrackerChannel::new(channel_id.clone(), config) },
                );
                Some(ch.clone())
            })
            .unwrap()
    }

    // repositoryからchannelを削除する
    // これは即時TrackerChannelが無くなることを示すわけではない。ChannelService内でメッセージを削除されることになる
    // いずれかのスレッドで保持されていたTrackerChannel構造体は保持されたままになっており、ChannelServiceを参照する操作が行えなくなる
    fn remove(&mut self, channel_id: &GnuId) {
        info!(root_session_id=?self.root_session_id_, ?channel_id, "Remove Channel");
        self.channels_.remove(channel_id).map(|ch| {
            ch.stop();
        });
    }
}

#[derive(Debug, Clone)]
struct TrackerRepository {
    internal_: Arc<RwLock<InternalRepository>>,
}

impl Repository<TrackerChannel> for TrackerRepository {
    fn new() -> Self {
        let internal_ = Arc::new(RwLock::new(InternalRepository::new(
            GnuId::new(),
            GnuId::new(),
        )));
        Self { internal_ }
    }

    fn session_id(&self) -> GnuId {
        self.internal_
            .read()
            .unwrap_or_else(rwlock_read_poisoned)
            .session_id()
    }

    fn broadcast_id(&self) -> GnuId {
        self.internal_
            .read()
            .unwrap_or_else(rwlock_read_poisoned)
            .broadcast_id()
    }

    fn get(&self, channel_id: &GnuId) -> Option<TrackerChannel> {
        self.internal_
            .read()
            .unwrap_or_else(rwlock_read_poisoned)
            .get(channel_id)
    }

    fn get_or_create(
        &self,
        channel_id: &GnuId,
        config: <TrackerChannel as Channel>::Config,
    ) -> TrackerChannel {
        // OPTIMIZE: writeは重いからなぁ
        self.internal_
            .write()
            .unwrap_or_else(rwlock_write_poisoned)
            .get_or_create(channel_id, config)
    }

    fn remove(&self, channel_id: &GnuId) {
        self.internal_
            .write()
            .unwrap_or_else(rwlock_write_poisoned)
            .remove(channel_id);
    }
}

#[cfg(test)]
mod t {
    use std::time::Duration;

    use peercast_re::{pcp::GnuId, util::mutex_poisoned};

    use crate::{
        channel::{
            implement::{
                ChannelServiceState, TrackerChannel, TrackerChannelConfig, TrackerRepository,
            },
            Channel, Repository,
        },
        test_helper::*,
    };

    #[tokio::test]
    async fn test_channel() {
        init_logger("debug");
        let channel_id = GnuId::new();
        let config = TrackerChannelConfig {
            tracker_session_id: GnuId::new(),
            tracker_broadcast_id: GnuId::new(),
        };

        let mut channel = TrackerChannel::new(channel_id, config);
        let mut state_tr = channel
            .service
            .lock()
            .unwrap_or_else(mutex_poisoned)
            .state_tr
            .clone();
        state_tr.changed().await;
        let state = state_tr.borrow_and_update().clone();
        assert_eq!(state, ChannelServiceState::Start);

        channel.stop();

        state_tr.changed().await;
        let state = state_tr.borrow_and_update().clone();
        assert_eq!(state, ChannelServiceState::Finished);
    }

    #[tokio::test]
    async fn test_tracker_repository() {
        init_logger("info");
        let repo = TrackerRepository::new();

        let id = GnuId::new();
        let config = TrackerChannelConfig {
            tracker_session_id: GnuId::new(),
            tracker_broadcast_id: GnuId::new(),
        };
        let ch = repo.get_or_create(&id, config.clone());
        assert_eq!(ch.config(), config);

        let mut state_tr = ch
            .service
            .lock()
            .unwrap_or_else(mutex_poisoned)
            .state_tr
            .clone();
        state_tr.changed().await;
        let state = state_tr.borrow_and_update().clone();
        assert_eq!(state, ChannelServiceState::Start);

        repo.remove(&id);

        state_tr.changed().await;
        let state = state_tr.borrow_and_update().clone();
        assert_eq!(state, ChannelServiceState::Finished);
    }
}
