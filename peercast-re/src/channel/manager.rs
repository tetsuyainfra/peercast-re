#![allow(dead_code)]
use libpeercast_re::pcp::Atom;
use tokio::sync::{mpsc::UnboundedReceiver, oneshot, watch};

use crate::prelude::*;
use crate::repository::Channel;

#[derive(Debug)]
pub(super) enum Message {
    // 上流への接続
    AddUpStream,
    // Rtmpなどのソースストリーム接続
    AddSourceStream(String),
    // 下流への接続
    AddDownStream,
    // 視聴用の接続
    AddSubscriber(oneshot::Sender<watch::Receiver<SubscriberMessage>>),

    GoDownAtom(Atom),
    GoUpAtom(Atom),
}

#[derive(Debug)]
pub enum SubscriberMessage {
    Connection(),
}

pub(super) type ChannelManagerSender = tokio::sync::mpsc::UnboundedSender<Message>;

#[derive(Debug)]
pub(super) struct ChannelManager {
    reciever: UnboundedReceiver<Message>,
    //
    upstreams: Vec<()>,
    //
    srcstream: Vec<()>,
    //
    downstreams: Vec<()>,
    //
    subscribers: Vec<()>,
    subscribers_sender: watch::Sender<SubscriberMessage>,
    subscribers_reciever: watch::Receiver<SubscriberMessage>,
}

impl ChannelManager {
    pub(super) fn new(reciever: UnboundedReceiver<Message>) -> Self {
        let (tx, rx) = watch::channel(SubscriberMessage::Connection());
        Self {
            reciever,
            upstreams: Default::default(),
            srcstream: Default::default(),
            downstreams: Default::default(),

            subscribers: Default::default(),
            subscribers_sender: tx,
            subscribers_reciever: rx,
        }
    }

    pub(super) async fn start(mut self, channel: super::ReChannel) {
        loop {
            tokio::select! {
                Some(msg) = self.reciever.recv() => {
                    self.handle_message(msg, &channel);
                }
                // 他の非同期イベントもここで処理可能
            }
        }
    }

    fn handle_message(&mut self, msg: Message, channel: &super::ReChannel) {
        match msg {
            Message::AddUpStream => {
                // 上流への接続を追加するロジックをここに実装
                println!("Adding upstream connection to channel: {:?}", channel.id());
            }
            Message::AddSourceStream(src_addr) => {
                // ソースストリームへの接続を追加するロジックをここに実装
                println!("Adding source stream connection to channel: {:?}, src_addr: {}", channel.id(), src_addr);
            }
            Message::AddDownStream => todo!(),
            Message::AddSubscriber(tx) => {
                if let Err(e) = tx.send(self.subscribers_reciever.clone()) {
                    error!("Failed to send subscriber receiver: {:?}", e);
                }
            }
            Message::GoDownAtom(atom) => {
                self.update_from_upstream(&atom);
                self.send_downstream(&atom);
            }
            Message::GoUpAtom(atom) => {
                self.update_from_downstream(&atom);
                self.send_upstream(&atom);
            }
        }
    }

    fn add_subscriber(&mut self) {
        // 視聴者を追加するロジックをここに実装
        println!("Adding subscriber. Total subscribers: {}", self.subscribers.len() + 1);
        self.subscribers.push(());
        let _ = self.subscribers_sender.send(SubscriberMessage::Connection());
    }

    fn update_from_upstream(&self, _atom: &Atom) {
        // 上流からAtomを受信した際の処理をここに実装
    }

    fn update_from_downstream(&self, _atom: &Atom) {
        // 下流からAtomを受信した際の処理をここに実装
    }

    fn send_downstream(&self, _atom: &Atom) {
        // 下流にAtomを送信するロジックをここに実装
    }
    fn send_upstream(&self, _atom: &Atom) {
        // 上流にAtomを送信するロジックをここに実装
    }
}
