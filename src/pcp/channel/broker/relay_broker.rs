use core::panic;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use async_trait::async_trait;
use axum::error_handling::future;
use bytes::Bytes;
use futures_util::{
    future::{select_all, BoxFuture},
    FutureExt,
};
use thiserror::Error;
use tokio::{sync::mpsc, task::JoinHandle};
use tracing::{debug, error, trace};

use crate::{
    pcp::{Atom, ChannelInfo, ChannelType, GnuId, TrackInfo},
    util::{util_mpsc::mpsc_send, Shutdown},
    ConnectionId,
};

use super::{BrokerError, ChannelBrokerMessage, ChannelBrokerWorker, ChannelMessage};

#[async_trait]
impl ChannelBrokerWorker for RelayBrokerWorker {
    fn new(
        channel_id: GnuId,
        channel_info: Arc<RwLock<Option<ChannelInfo>>>,
        track_info: Arc<RwLock<Option<TrackInfo>>>,
        //
        shutdown_rx: mpsc::UnboundedReceiver<()>,
    ) -> Self {
        RelayBrokerWorker::new(channel_id, channel_info, track_info, shutdown_rx)
    }

    async fn start(
        mut self,
        manager_receiver: mpsc::UnboundedReceiver<ChannelBrokerMessage>,
    ) -> Result<(), BrokerError> {
        self.start(manager_receiver).await
    }
}

// Worker内で利用するreceiverをラップしたクラス
#[derive(Debug)]
enum FutureResult {
    Disconnection {
        connection_id: ConnectionId,
    },
    MessageReceived {
        receiver: mpsc::UnboundedReceiver<ChannelBrokerMessage>,
        message: Option<ChannelBrokerMessage>,
    },
}

// Relayを開始して一番最初に送られてくる｜送るAtom
#[derive(Debug)]
struct HeadData {
    atom: Atom,
    pos: u32,
    payload: Bytes,
}

/// Relay専用のブローカーワーカータスク
pub(super) struct RelayBrokerWorker {
    channel_id: GnuId,
    channel_info: Arc<RwLock<Option<ChannelInfo>>>,
    track_info: Arc<RwLock<Option<TrackInfo>>>,
    shutdown_rx: mpsc::UnboundedReceiver<()>,
    //
    sender_by_connection_id: HashMap<ConnectionId, mpsc::UnboundedSender<ChannelMessage>>,
    new_disconnect_futures: Vec<BoxFuture<'static, FutureResult>>,
    new_disconnections: Vec<(ConnectionId, mpsc::UnboundedReceiver<()>)>,
    //
    head_data: Option<HeadData>,
}

impl RelayBrokerWorker {
    fn new(
        channel_id: GnuId,
        channel_info: Arc<RwLock<Option<ChannelInfo>>>,
        track_info: Arc<RwLock<Option<TrackInfo>>>,
        shutdown_rx: mpsc::UnboundedReceiver<()>,
    ) -> Self {
        Self {
            channel_id,
            channel_info,
            track_info,
            shutdown_rx,
            //
            sender_by_connection_id: Default::default(),
            new_disconnect_futures: Default::default(),
            new_disconnections: Default::default(),
            head_data: None,
        }
    }

    fn cleanup_connection(&mut self, connection_id: ConnectionId) {
        println!("Stream manager is removing connection id {}", connection_id);

        self.sender_by_connection_id.remove(&connection_id);
        // if let Some(key) = self.key_by_connection_id.remove(&connection_id) {
        //     if let Some(players) = self.players_by_key.get_mut(&key) {
        //         players.remove(&connection_id);
        //     }

        //     if let Some(details) = self.publish_details.get_mut(&key) {
        //         if details.connection_id == connection_id {
        //             self.publish_details.remove(&key);
        //         }
        //     }
        // }
    }

    // 送られてきたrecieverをラップするselect_allできるようにする
    async fn wait_for_client_disconnection(
        connection_id: ConnectionId,
        mut receiver: mpsc::UnboundedReceiver<()>,
    ) -> FutureResult {
        // The channel should only be closed when the client has disconnected
        while let Some(()) = receiver.recv().await {}

        FutureResult::Disconnection { connection_id }
    }

    // fn new_disconnect_futures

    async fn start(
        mut self,
        mut manager_receiver: mpsc::UnboundedReceiver<ChannelBrokerMessage>,
    ) -> Result<(), BrokerError> {
        debug!("RelayBrokerWorker START CID:{:.07}", self.channel_id);
        async fn new_receiver_future(
            mut receiver: mpsc::UnboundedReceiver<ChannelBrokerMessage>,
        ) -> FutureResult {
            let result = receiver.recv().await;
            FutureResult::MessageReceived {
                receiver,
                message: result,
            }
        }

        // select_allはFutureのリストを処理して、最初にreadyになったfutureの値とindexを返す(loop内 futures.await)
        // https://docs.rs/futures/latest/futures/future/fn.select_all.html
        // messageの受信とdisconnectionを並列して処理しなくてはならず、messageの受信はともかく、connection切断は複数あり得るのでこうなってる
        let mut futures = select_all(vec![new_receiver_future(manager_receiver).boxed()]);

        loop {
            let (result, _index, remaining_futures) = futures.await;
            let mut new_futures = Vec::from(remaining_futures);

            // trace!(message = ?result);
            match result {
                FutureResult::MessageReceived { receiver, message } => {
                    match message {
                        Some(message) => self.handle_message(message),
                        None => break,
                    };
                    new_futures.push(new_receiver_future(receiver).boxed()); // メッセージを処理したら、新たにリストに処理待ちする
                }

                FutureResult::Disconnection { connection_id } => {
                    self.cleanup_connection(connection_id)
                }
            }

            for future in self.new_disconnect_futures.drain(..) {
                new_futures.push(future);
            }

            futures = select_all(new_futures);
        }

        // Shutdown(終了処理)
        debug!("RelayBrokerWorker FINISH CID:{:.07}", self.channel_id);

        Ok(())
    }

    fn handle_message(&mut self, message: ChannelBrokerMessage) {
        match message {
            ChannelBrokerMessage::NewConnection {
                connection_id,
                sender,
                disconnection,
            } => todo!(),
            ChannelBrokerMessage::UpdateChannelInfo { info, track } => todo!(),
            ChannelBrokerMessage::ArrivedChannelHead {
                atom,
                payload,
                pos,
                info,
                track,
            } => self.handle_arrived_channel_head(atom, pos, payload, info, track),
            ChannelBrokerMessage::ArrivedChannelData {
                atom,
                payload,
                pos,
                continuation,
            } => self.handle_arrived_channel_data(atom, pos, payload, continuation),
            ChannelBrokerMessage::AtomBroadcast { direction, atom } => todo!(),
            ChannelBrokerMessage::BroadcastEvent(_) => todo!(),
        }
    }

    fn handle_new_connection(
        &mut self,
        connection_id: ConnectionId,
        mut sender: mpsc::UnboundedSender<ChannelMessage>,
        disconnection: mpsc::UnboundedReceiver<()>,
    ) {
        // metadataが有れば送っておく
        if (self.head_data.is_some()) {
            let HeadData { atom, pos, payload } = self.head_data.as_ref().unwrap();
            let info = self.channel_info.read().unwrap().clone();
            let track = self.track_info.read().unwrap().clone();
            mpsc_send(
                &mut sender,
                ChannelMessage::RelayChannelHead {
                    atom: atom.clone(),
                    pos: *pos,
                    payload: payload.clone(),
                    info,
                    track,
                },
            );
        }

        match self.sender_by_connection_id.insert(connection_id, sender) {
            Some(_sender) => {
                error!(?connection_id, "connection id never overlap.");
                panic!("connection id never overlap.");
            }
            None => {}
        };
        self.new_disconnect_futures
            .push(Self::wait_for_client_disconnection(connection_id, disconnection).boxed());
    }

    fn handle_arrived_channel_head(
        &mut self,
        atom: Atom,
        pos: u32,
        payload: Bytes,
        info: Option<ChannelInfo>,
        track: Option<TrackInfo>,
    ) {
        {
            let mut lock_info = self.channel_info.write().unwrap();
            let mut lock_track = self.track_info.write().unwrap();
            if info.is_some() {
                *lock_info = info.clone();
            }
            if track.is_some() {
                *lock_track = track.clone();
            }
        }

        if self.head_data.is_none() {
            trace!("BROKER UPDATE HAED_DATA CID:{:.07}", self.channel_id);
            trace!(HEAD_DATA_ATOM=?atom);
            self.head_data = Some(HeadData { atom, pos, payload })
        } else {
            let head_data = self.head_data.as_mut().unwrap();
            head_data.pos = pos;
            head_data.payload = payload;
        }

        let HeadData { atom, pos, payload } = self.head_data.as_ref().unwrap();
        self.send_listener(ChannelMessage::RelayChannelHead {
            atom: atom.clone(),
            pos: pos.clone(),
            payload: payload.clone(),
            info: None,
            track: None,
        })
    }
    fn handle_arrived_channel_data(
        &mut self,
        atom: Atom,
        pos: u32,
        payload: Bytes,
        continuation: bool,
    ) {
        if self.head_data.is_none() {
            panic!("Headが送られてくる前にデータが来るのはおかしい");
        }
        self.send_listener(ChannelMessage::RelayChannelData {
            atom: atom.clone(),
            pos,
            payload,
            continuation,
        });
    }

    /// brokerをlistenしているRelay, Readerにデータを配信する
    fn send_listener(&self, message: ChannelMessage) {
        for (id, sender) in &self.sender_by_connection_id {
            mpsc_send(sender, message.clone());
        }
    }
}

#[cfg(test)]
mod t {
    use super::*;
    use crate::test_helper;

    #[crate::test]
    async fn worker() {
        test_helper::init_logger("debug");

        let info = Arc::new(RwLock::new(None));
        let track = Arc::new(RwLock::new(None));
        let (shutdown_tx, shutdown_rx) = mpsc::unbounded_channel();
        let worker = RelayBrokerWorker::new(
            GnuId::new(),
            Arc::clone(&info),
            Arc::clone(&track),
            shutdown_rx,
        );

        let (manager_tx, manager_rx) = mpsc::unbounded_channel();
        let handle = tokio::spawn(worker.start(manager_rx));

        drop(manager_tx);
        handle.await;
    }
}
