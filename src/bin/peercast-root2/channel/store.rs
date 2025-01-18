use std::{
    collections::HashMap,
    marker::PhantomData,
    sync::{Arc, Mutex, RwLock},
};

use peercast_re::{
    pcp::GnuId,
    util::{mutex_poisoned, rwlock_read_poisoned, rwlock_write_poisoned},
};
use tokio::{
    sync::mpsc::{self},
    task::JoinHandle,
};
use tracing::{info, trace};

use super::ChannelTrait;

//------------------------------------------------------------------------------
// Channel構造体を実際に保管するStore
//
#[derive(Debug)]
struct Store<C>
where
// C: Send + Sync,
// Self: Send + Sync,
{
    root_session_id_: GnuId,
    root_broadcast_id_: GnuId,
    channels_: RwLock<HashMap<GnuId, C>>,
    watcher_sender_: mpsc::UnboundedSender<ChannelWatcherMessage>,
}

impl<C> Store<C>
where
    // Self: 'static,
    C: ChannelTrait,
{
    // fn new() -> Store<C> {
    fn new(
        root_session_id_: GnuId,
        root_broadcast_id_: GnuId,
        watcher_sender_: mpsc::UnboundedSender<ChannelWatcherMessage>,
    ) -> Self {
        let store = Store {
            channels_: Default::default(),
            root_session_id_,
            root_broadcast_id_,
            watcher_sender_,
        };
        store
    }

    pub fn session_id(&self) -> GnuId {
        self.root_session_id_
    }

    pub fn broadcast_id(&self) -> GnuId {
        self.root_broadcast_id_
    }

    pub fn get(&self, channel_id: &GnuId) -> Option<C> {
        self.channels_
            .read()
            .unwrap_or_else(rwlock_read_poisoned)
            .get(channel_id)
            .map(|c| c.clone())
    }

    fn get_or_create(&self, channel_id: &GnuId, config: C::Config) -> C {
        self.get(channel_id)
            .or_else(|| {
                //
                let mut channels = self.channels_.write().unwrap_or_else(rwlock_write_poisoned);

                let ch = channels.entry(channel_id.clone()).or_insert_with(|| {
                    C::new(
                        self.root_session_id_.clone(),
                        self.root_broadcast_id_.clone(),
                        channel_id.clone(),
                        config,
                        self.watcher_sender_.clone(),
                    )
                });
                Some(ch.clone())
            })
            .unwrap()
    }

    fn remove(&self, channel_id: &GnuId) -> Option<C> {
        self.channels_
            .write()
            .unwrap_or_else(rwlock_write_poisoned)
            .remove(channel_id)
    }
}

//------------------------------------------------------------------------------
/// Watcherへ送るメッセージ
///
pub enum ChannelWatcherMessage {
    RemoveChannel(GnuId),
}

//------------------------------------------------------------------------------
/// Channelインスタンスを監視(と通信）してStoreからの削除を実行するスレッドを管理するための構造体
///
#[derive(Debug)]
struct ChannelWatcher<C> {
    // sender: UnboundedSender<()>,
    handle_: Option<JoinHandle<()>>,
    cancel_token_: tokio_util::sync::CancellationToken,
    _channel_type: PhantomData<C>,
}

impl<C> ChannelWatcher<C>
where
    Self: 'static,
    C: ChannelTrait,
{
    fn new(
        id: GnuId,
        store_: Arc<Store<C>>,
        msg_receiver: mpsc::UnboundedReceiver<ChannelWatcherMessage>,
    ) -> ChannelWatcher<C> {
        let cancel_token_ = tokio_util::sync::CancellationToken::new();
        let handle_ = Some(tokio::spawn(Self::watcher_main(
            id,
            store_,
            msg_receiver,
            cancel_token_.clone(),
        )));
        Self {
            // sender,
            handle_,
            _channel_type: PhantomData,
            cancel_token_,
        }
    }

    fn stop(&mut self) {
        trace!("CALL STOP WATCHER");
        self.cancel_token_.cancel();
        // match self.handle.take() {
        //     Some(h) => h.abort(),
        //     None => (),
        // }
    }
    async fn wait_stop(&mut self) {
        self.stop();
        match self.handle_.take() {
            None => {}
            Some(handle) => {
                let _ = handle.await;
            }
        }
    }

    async fn watcher_main(
        id: GnuId,
        store: Arc<Store<C>>,
        mut tx: mpsc::UnboundedReceiver<ChannelWatcherMessage>,
        cancel_token: tokio_util::sync::CancellationToken,
    ) {
        info!(?id, "START WATCHER");

        'main: loop {
            tokio::select! {
                Some(msg) = tx.recv() => {
                    match msg {
                        ChannelWatcherMessage::RemoveChannel(channel_id) => {
                            match store.get(&channel_id) {
                                None => continue 'main,
                                Some(mut channel) => {
                                    channel.before_remove(channel_id);
                                },
                            };
                            tokio::task::yield_now().await;
                            match store.remove(&channel_id) {
                                None => continue 'main,
                                Some(mut channel) => {
                                    channel.after_remove(channel_id);
                                },
                            };
                        },
                    }
                },
                _ = cancel_token.cancelled() => {
                    break;
                },
                else => {
                    info!(?id, "WATCHER MSG SENDER IS ALL CLOSED");
                    break;
                }
            }
        }
        info!(?id, "STOP WATCHER");
    }
}

//------------------------------------------------------------------------------
// 外部からStoreを操作するための構造体およびインターフェイス
//
#[derive(Debug, Clone)]
pub struct ChannelStore<C> {
    store_: Arc<Store<C>>,
    watcher_: Arc<Mutex<ChannelWatcher<C>>>,
}

impl<C> ChannelStore<C>
where
    Self: 'static,
    C: ChannelTrait,
{
    pub fn new(self_session_id: Option<GnuId>, self_broadcast_id: Option<GnuId>) -> Self {
        let root_session_id = self_session_id.unwrap_or_else(|| GnuId::new());
        let root_broadcast_id = self_broadcast_id.unwrap_or_else(|| GnuId::new());
        let (watcher_tx, watcher_rx) = tokio::sync::mpsc::unbounded_channel();

        let store_ = Arc::new(Store::new(root_session_id, root_broadcast_id, watcher_tx));
        let watcher_ = ChannelWatcher::new(root_session_id, store_.clone(), watcher_rx);

        info!(session_id = ?store_.session_id(), broadcast_id = ?store_.broadcast_id(), "CREATED STORE");
        ChannelStore {
            store_,
            watcher_: Arc::new(Mutex::new(watcher_)),
        }
    }

    pub fn watcher_stop(&self) {
        self.watcher_.lock().unwrap_or_else(mutex_poisoned).stop();
    }
    async fn wait_watcher_stop(&self) {
        let mut watcher = self.watcher_.lock().unwrap_or_else(mutex_poisoned);
        watcher.wait_stop().await;
    }

    pub fn get(&self, channel_id: &GnuId) -> Option<C> {
        self.store_.get(channel_id)
    }

    fn get_or_create(&self, channel_id: &GnuId, config: C::Config) -> C {
        self.store_.get_or_create(channel_id, config)
    }

    // MEMO: Watcherへのレシーバーの入れ替えをどうすればよいか検討できていない
    // #[cfg(not(all()))]
    // pub fn watcher_restart(&self) {
    //     match self.watcher_.lock() {
    //         Ok(mut watcher) => {
    //             watcher.stop();
    //             *watcher = ChannelWatcher::new(self.store_.clone());
    //         }
    //         Err(_) => todo!(),
    //     }
    // }
}

#[cfg(test)]
mod t {
    

    use peercast_re::pcp::GnuId;
    

    use crate::channel::{
        tracker_channel::{TrackerChannel, TrackerChannelConfig},
        ChannelTrait,
    };

    use super::ChannelStore;

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn channel_control() {
        let store: ChannelStore<TrackerChannel> = ChannelStore::new(None, None);
        let id = GnuId::new();
        let config = TrackerChannelConfig {};
        let ch1 = store.get_or_create(&id, config);
        store.watcher_stop();
        store.wait_watcher_stop().await;

        // store.watcher_restart();

        // let id = GnuId::new();
        // assert_eq!(None, store.get(&id));

        // let ch = store.get_or_create(&id, TrackerChannelConfig {});
        // ch.stop();
        // assert_eq!(None, store.get(&id));
    }

    // #[tokio::test(start_paused = false)]
    // async fn paused_time() {
    //     // tokio::time::pause();
    //     let start = std::time::Instant::now();
    //     tokio::time::sleep(Duration::from_millis(500)).await;
    //     println!("{:?}ms", start.elapsed().as_millis());
    // }
}
