// src/pcp/connection/mod.rs

use std::{
    collections::VecDeque,
    io,
    net::SocketAddr,
    sync::{mpsc::Receiver, Arc},
};

use bytes::{Bytes, BytesMut};
use futures_util::FutureExt;
use thiserror::Error;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    net::TcpStream,
    sync::{mpsc, oneshot},
};
use tracing::{error, warn};

use crate::{
    error,
    pcp::{Atom, ChannelManager},
    util::{util_mpsc::send, ConnectionProtocol},
};

use super::{
    ChannelBrokerMessage, ConnectionAction, ConnectionError, ServerSession, ServerSessionResult,
};

pub struct InConnection {
    connection_id: u64,
    remote: SocketAddr,
    channel_manager: Arc<ChannelManager>,
    //
    session: Option<ServerSession>,
}

impl InConnection {
    pub fn new(
        connection_id: u64,
        remote: SocketAddr,
        channel_manager: Arc<ChannelManager>,
        protocol: ConnectionProtocol,
    ) -> Self {
        Self {
            connection_id,
            remote,
            channel_manager,
            session: None,
        }
    }

    //--------------------------------------------------------------------------------
    // Incomming
    //
    pub async fn start_negotiation(mut self, mut stream: TcpStream) -> Result<(), ConnectionError> {
        let (recieve_bytes,) = self.handhsake_incomming(&mut stream);

        // Brokerとのメッセ―ジのやり取りをするチャンネル
        let (message_sender, mut message_receiver) = mpsc::unbounded_channel();
        // この関数から脱出すると終了を知らせる為にチャンネルを作っておく
        let (_disconnection_sender, disconnection_receiver) = oneshot::channel();
        let mesg = ChannelBrokerMessage::NewConnection {
            connection_id: self.connection_id,
            sender: message_sender,
            disconnection: disconnection_receiver,
        };

        // ChannelBrokerにコネクション開始のメッセージを送る
        // if !send(&channel_broker_sender, message) {
        //     return Ok(());
        // }
        // ここまででBroker側の最低限の準備完了

        // R/Wに分けてメッセージを処理
        let (stream_reader, stream_writer) = tokio::io::split(stream);

        // Reader
        let (read_bytes_sender, mut read_bytes_receiver) = mpsc::unbounded_channel();
        tokio::spawn(Self::connection_reader(
            self.connection_id,
            stream_reader,
            read_bytes_sender,
        ));

        // Writer
        let (mut write_bytes_sender, write_bytes_receiver) = mpsc::unbounded_channel();
        tokio::spawn(Self::connection_writer(
            self.connection_id,
            stream_writer,
            write_bytes_receiver,
        ));

        // Sessionを作成する
        let (session, mut results) = ServerSession::new();
        //.map_err()
        self.session = Some(session);

        let remaining_bytes_results = self
            .session
            .as_mut()
            .unwrap()
            .handle_input(&recieve_bytes)?;
        // .map_err(|x| format!("Failed to handle input: {:?}", x))?;

        results.extend(remaining_bytes_results);

        loop {
            // ?
            // let action = self.handle_session_results(&mut results, write_bytes_sender)?;

            // if action == ConnectionAction::Disconnect {
            //     break;
            // }
        }

        Ok(())
    }

    fn handhsake_incomming(&mut self, stream: &mut TcpStream) -> (Bytes,) {
        todo!()
    }

    fn handle_session_results(
        &self,
        results: &mut Vec<ServerSessionResult>,
        sender: mpsc::UnboundedSender<Atom>,
    ) -> Result<ConnectionAction, ConnectionError> {
        todo!()
    }

    //------------------------------------------------------------------------------
    // 以下クラスメソッド(相当)
    //
    async fn connection_reader(
        connection_id: u64,
        mut reader_stream: ReadHalf<TcpStream>,
        mut sender: mpsc::UnboundedSender<Bytes>,
    ) -> Result<(), io::Error> {
        let mut buffer = BytesMut::with_capacity(4096);
        loop {
            let bytes_read = reader_stream.read_buf(&mut buffer).await?;

            if bytes_read == 0 {
                break;
            }

            let bytes = buffer.split_off(bytes_read);
            if !send(&sender, buffer.freeze()) {
                break;
            }

            buffer = bytes;
        }

        warn!("Connection {}: Reader disconnected", connection_id);
        Ok(())
    }
    async fn connection_writer(
        connection_id: u64,
        mut writer: WriteHalf<TcpStream>,
        mut reciever: mpsc::UnboundedReceiver<Atom>,
    ) {
        let mut send_queue = VecDeque::new();
        loop {
            let Some(atom) = reciever.recv().await else {
                break;
            };

            send_queue.push_back(atom);

            // サーバークライアント間の帯域が狭い時どんどんデータがたまることになる
            // mpscに保留中のパケット量を知る良い機能がない
            // 保留中のパケットを全部受け取って、キューに入れる。
            // で、キューが大きすぎる場合、オプションのパケットを無視する
            while let Some(Some(atom)) = reciever.recv().now_or_never() {
                send_queue.push_back(atom);
            }

            // で
            // let mut send_optional_packets = true;
            // if send_queue.len() > BACKLOG_THRESHOLD {
            //     println!(
            //         "Connection {}: Too many pending packets, dropping optional ones",
            //         connection_id
            //     );
            //     send_optional_packets = false;
            // }

            for atom in send_queue.drain(..) {
                // if send_optional_packets || !packet.can_be_dropped {
                //     stream.write_all(packet.bytes.as_ref()).await?;
                // }
                // stream.write_all(packet.bytes.as_ref()).await?;
                panic!("please coding write packet");
            }
        }
        warn!("Connection {}: Writer disconnected", connection_id);
    }
}
