#![allow(unused)]
use std::{
    collections::HashMap,
    fmt::Debug,
    marker::PhantomData,
    ops::Deref,
    pin::Pin,
    process::exit,
    result,
    sync::{Arc, Mutex, RwLock},
};

use futures_util::{future, FutureExt};
use peercast_re::{
    pcp::{Atom, GnuId},
    rtmp::connection,
    util::{rwlock_read_poisoned, rwlock_write_poisoned},
    ConnectionId,
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info};

trait ChannelService: Send + Sync + 'static {
    fn atom_arrived(&mut self, atom: Atom, direction: FlowDirection) {
        info!(?atom, "msg_arrived");
    }

    fn disconnect_preprocess(&mut self, connection_id: ConnectionId) {
        info!(?connection_id, "disconnect_preprocess()");
    }
}

#[derive(Debug, Clone)]
pub struct Channel {
    id: GnuId,
    sender: tokio::sync::mpsc::UnboundedSender<ChannelServiceMessage>,
    runner: Arc<Mutex<ChannelServiceRunner>>,
}

impl Channel {
    fn new(id: GnuId, svc: Box<dyn ChannelService>) -> Self {
        let (runner, sender) = ChannelServiceRunner::run(id, svc);
        Self {
            id,
            sender,
            runner: Arc::new(runner.into()),
        }
    }

    fn subscribe(
        &mut self,
    ) -> (
        tokio::sync::mpsc::UnboundedReceiver<ConnectionMessage>,
        tokio::sync::mpsc::UnboundedSender<()>,
    ) {
        let (tx, tr) = tokio::sync::mpsc::unbounded_channel::<ConnectionMessage>();
        let (disconnection_tx, disconection_tr) = tokio::sync::mpsc::unbounded_channel::<()>();

        let msg = ChannelServiceMessage::NewConnection {
            connection_id: ConnectionId::new(),
            sender: tx,
            disconnection: disconection_tr,
        };
        self.sender
            .send(msg)
            .expect("ChannelServiceRunnerでエラーが起きている");
        (tr, disconnection_tx)
    }
}

#[derive(Debug)]
struct ChannelMessageReciever {}

#[derive(Debug)]
struct ChannelServiceRunner {
    handle: tokio::task::JoinHandle<()>,
    // msg_sender: tokio::sync::mpsc::UnboundedSender<ChannelServiceMessage>,
    status: tokio::sync::watch::Receiver<ChannelServiceStatus>,
    cancel_token: tokio_util::sync::CancellationToken,
}

impl ChannelServiceRunner {
    fn force_stop(&self) {
        self.cancel_token.cancel();
    }

    fn run(
        id: GnuId,
        mut svc: Box<dyn ChannelService>,
    ) -> (
        ChannelServiceRunner,
        tokio::sync::mpsc::UnboundedSender<ChannelServiceMessage>,
    ) {
        let (msg_sender, msg_reciever) = tokio::sync::mpsc::unbounded_channel();
        let (status_tx, status_tr) =
            tokio::sync::watch::channel(ChannelServiceStatus::Initializing);
        let cancel_token = CancellationToken::new();

        let handle = tokio::spawn(Self::main(
            id,
            svc,
            msg_reciever,
            status_tx,
            cancel_token.clone(),
        ));

        let runner = ChannelServiceRunner {
            handle,
            // msg_sender,
            status: status_tr,
            cancel_token,
        };
        (runner, msg_sender)
    }

    async fn main(
        id: GnuId,
        mut svc: Box<dyn ChannelService>,
        msg_reciever: UnboundedReceiver<ChannelServiceMessage>,
        status: tokio::sync::watch::Sender<ChannelServiceStatus>,
        cancel_token: tokio_util::sync::CancellationToken,
    ) -> () {
        info!(?id, "START: ChannelServiceRunner");
        status.send(ChannelServiceStatus::Idle).unwrap();

        async fn new_receiver_future(
            mut receiver: UnboundedReceiver<ChannelServiceMessage>,
        ) -> FutureResult {
            let result = receiver.recv().await;
            FutureResult::MessageReceived {
                receiver,
                message: result,
            }
        }

        async fn cancel_token_future(
            cancel_token: tokio_util::sync::CancellationToken,
        ) -> FutureResult {
            cancel_token.cancelled().await;
            FutureResult::ForceCancelRequested {}
        }

        async fn wait_for_client_disconnection(
            connection_id: ConnectionId,
            mut receiver: UnboundedReceiver<()>,
        ) -> FutureResult {
            // The channel should only be closed when the client has disconnected
            while let Some(()) = receiver.recv().await {}

            FutureResult::Disconnection { connection_id }
        }

        let mut futures = future::select_all(vec![
            new_receiver_future(msg_reciever).boxed(),
            cancel_token_future(cancel_token).boxed(),
        ]);

        'main: loop {
            let (result, _idx, remaining_futures) = futures.await;
            let mut new_futures = Vec::from(remaining_futures);
            //

            let mut new_disconnect_futures: Vec<future::BoxFuture<'static, FutureResult>> =
                Vec::new();
            // Disconnectionをこちらに移譲したい
            // すると接続のプロセスもこちらでするべき
            match result {
                FutureResult::MessageReceived { receiver, message } => {
                    match message {
                        Some(msg) => {
                            info!(?msg, "ChannelServiceMessage is arrived");
                            match msg {
                                ChannelServiceMessage::NewConnection {
                                    connection_id,
                                    sender,
                                    disconnection,
                                } => new_disconnect_futures.push(
                                    wait_for_client_disconnection(connection_id, disconnection)
                                        .boxed(),
                                ),
                                ChannelServiceMessage::NewAtom { atom, direction } => {
                                    svc.atom_arrived(atom, direction)
                                }
                            }
                        }
                        None => {
                            info!(?id, "ChannelServiceRunner sender is all gone");
                            break 'main;
                        }
                    }
                    new_futures.push(new_receiver_future(receiver).boxed());
                }
                FutureResult::Disconnection { connection_id } => {
                    svc.disconnect_preprocess(connection_id);
                }
                FutureResult::ForceCancelRequested {} => {
                    // ForceStopとそうじゃない場合を分けた方がよくないか？
                    break 'main;
                }
            }

            for future in new_disconnect_futures.drain(..) {
                new_futures.push(future);
            }

            // 次のループへ持ち越し
            futures = future::select_all(new_futures);
        }

        status.send(ChannelServiceStatus::Stop).unwrap();
        info!(?id, "STOP: ChannelServiceRunner");
    }
}

enum FutureResult {
    Disconnection {
        connection_id: ConnectionId,
    },
    MessageReceived {
        receiver: UnboundedReceiver<ChannelServiceMessage>,
        message: Option<ChannelServiceMessage>,
    },
    ForceCancelRequested {},
}

/// ChannelServiceRunnerおよびChannelServiceに通知するメッセージ
#[derive(Debug)]
enum ChannelServiceMessage {
    // これはServiceTaskでなにかしら実行される
    NewConnection {
        connection_id: ConnectionId,
        sender: tokio::sync::mpsc::UnboundedSender<ConnectionMessage>,
        disconnection: tokio::sync::mpsc::UnboundedReceiver<()>,
    },
    NewAtom {
        atom: Atom,
        direction: FlowDirection,
    },
}
#[derive(Debug)]
enum FlowDirection {
    // Upstream to Downstream
    UpToDown,
    // Downstream to Upstream
    DownToUp,
}

/// ChannelServiceに接続したConnectionが受け取るメッセージ
#[derive(Debug)]
enum ConnectionMessage {
    ConnectAccepted {},
}

// Trackerの時は
// IdleとRecievingシカナイハズ
#[derive(Debug, Clone, PartialEq)]
enum ChannelServiceStatus {
    Initializing,
    Idle,
    Searching,
    Connectiong,
    Recieving,
    Error,
    Stop,
}

type RepositoryStore = Arc<RwLock<HashMap<GnuId, Channel>>>;
#[derive(Debug)]
struct Repository<S> {
    channels: RepositoryStore,
    svc: S,
    // runner: RepositoryService,
}

impl<S> Repository<S>
where
    S: ChannelService + Clone + 'static,
{
    fn new(svc: S) -> Self {
        let channels = RepositoryStore::default();
        Self {
            channels: channels.clone(),
            svc,
            // runner: RepositoryService::new(channels),
        }
    }

    fn get(&self, id: &GnuId) -> Option<Channel> {
        self.channels
            .read()
            .unwrap_or_else(rwlock_read_poisoned)
            .get(id)
            .map(|c| c.clone())
    }

    fn create(&mut self, id: GnuId) -> Channel {
        self.channels
            .write()
            .unwrap_or_else(rwlock_write_poisoned)
            .entry(id)
            .or_insert_with_key(|id| {
                let svc_boxed = Box::new(self.svc.clone());
                Channel::new(id.clone(), svc_boxed)
            })
            .clone()
    }

    fn remove(&mut self, id: GnuId) -> bool {
        self.channels
            .write()
            .unwrap_or_else(rwlock_write_poisoned)
            .remove(&id)
            .map(|channel| {
                // HACKME: 別スレッドに削除を任せるようにしたい
                // self.runner.remove(channel);
                drop(channel);
                true
            })
            .is_some()
    }
}

#[cfg(test)]
mod ttt {
    use std::pin::Pin;
    use std::time::Duration;

    use futures_util::future;
    use futures_util::future::BoxFuture;
    use peercast_re::pcp::GnuId;
    use peercast_re::ConnectionId;
    use tokio::time::Sleep;
    use tracing::info;

    use crate::channel::channel::ChannelServiceStatus;
    use crate::channel::channel::Repository;
    use crate::test_helper;

    use super::Channel;
    use super::ChannelService;
    use super::ChannelServiceMessage;
    use super::FutureResult;

    #[derive(Debug, Clone)]
    struct S {
        count: usize,
    }

    impl ChannelService for S {
        fn atom_arrived(&mut self, atom: peercast_re::pcp::Atom, direction: super::FlowDirection) {
            info!(?atom, ?self.count, "msg_arrived");
            self.count += 1;
        }

        fn disconnect_preprocess(&mut self, connection_id: ConnectionId) {
            info!(?connection_id, "disconnect_preprocess()");
        }
    }

    #[tokio::test]
    async fn channel_graceful_shutdown() {
        test_helper::init_logger("DEBUG");
        let s = S { count: 0 };
        let mut ch = Channel::new(GnuId::new(), Box::new(s));
        let (mut status) = { ch.runner.lock().unwrap().status.clone() };
        status.changed().await;
        assert_eq!(
            ChannelServiceStatus::Idle,
            status.borrow_and_update().clone()
        );

        drop(ch);

        status.changed().await;
        assert_eq!(
            ChannelServiceStatus::Stop,
            status.borrow_and_update().clone()
        );
    }

    #[tokio::test]
    async fn channel_force_shutdown() {
        test_helper::init_logger("DEBUG");
        let s = S { count: 0 };
        let mut ch = Channel::new(GnuId::new(), Box::new(s));
        let (mut status, mut cancel_token) = {
            let lock = ch.runner.lock().unwrap();
            (lock.status.clone(), lock.cancel_token.clone())
        };
        status.changed().await;
        let x = status.borrow_and_update().clone();
        assert_eq!(ChannelServiceStatus::Idle, x);

        {
            ch.runner.lock().unwrap().force_stop();
        }
        status.changed().await;
        assert_eq!(
            ChannelServiceStatus::Stop,
            status.borrow_and_update().clone()
        );
    }
    #[tokio::test]
    async fn channel_subscribe() {
        test_helper::init_logger("DEBUG");
        let s = S { count: 0 };
        let mut ch = Channel::new(GnuId::new(), Box::new(s));
        let (sender, disconnect) = ch.subscribe();

        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    #[tokio::test]
    async fn test_repo() {
        test_helper::init_logger("debug");
        let mut repo = Repository::new(S { count: 0 });
        let id1 = GnuId::new();
        let mut ch1 = repo.create(id1.clone());
        let mut ch2 = repo.create(id1.clone());
        assert_eq!(ch1.id, ch2.id);
        let id3 = GnuId::new();
        let mut ch3 = repo.create(id3);
        assert_ne!(ch1.id, ch3.id);
        assert_ne!(ch2.id, ch3.id);

        let mut status = {
            let lock = ch1.runner.lock().unwrap();
            lock.status.clone()
        };
        status.changed().await;
        let x = status.borrow_and_update().clone();
        println!("status: {:?}", x);

        assert_eq!(repo.remove(GnuId::new()), false);
        assert_eq!(repo.remove(id1.clone()), true);
        assert_eq!(repo.remove(id3.clone()), true);
    }
}
