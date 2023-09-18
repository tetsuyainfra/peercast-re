use std::{
    collections::VecDeque,
    io::{self, BufRead},
    net::SocketAddr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use bytes::{Bytes, BytesMut};
use futures_util::FutureExt;
use tokio::{
    io::{AsyncReadExt, ReadHalf, WriteHalf},
    net::TcpStream,
    sync::{
        mpsc::{self, UnboundedReceiver},
        oneshot, watch,
    },
};
use tracing::{debug, warn};

use crate::pcp::{procedure, session, stream, Atom, ChannelInfo, ChannelManager, GnuId};

use super::{ConnectionError, ServerSession};

const CONNECTION_ID_START: u64 = 1_u64;
const CONNECTION_ID_COUTUP: u64 = 1_u64;

pub enum OutConnectionError {}

//--------------------------------------------------------------------------------
// Outgoing Connection
//
pub struct OutConnection {
    /// 'Outgoing' Connection ID
    connection_id: u64,
    // remote: SocketAddr,

    // use session
    session_id: GnuId,
    broadcast_id: GnuId,
    session: Option<ServerSession>,
    //
    tx: Option<mpsc::UnboundedSender<ChannelMessage>>,
}
pub enum ChannelMessage {
    Data,
}

impl OutConnection {
    pub fn new(session_id: GnuId, broadcast_id: GnuId) -> Self {
        static ID_COUNT: AtomicU64 = AtomicU64::new(CONNECTION_ID_START);
        let count = ID_COUNT.fetch_add(CONNECTION_ID_COUTUP, Ordering::SeqCst);

        Self {
            connection_id: count,
            // use session
            session_id,
            broadcast_id,
            session: None,
            //
            tx: None,
        }
    }

    pub async fn start_negotiation(mut self, mut stream: TcpStream) -> Result<(), ConnectionError> {
        let (stream, buffer) = self.handhsake_outgoing(stream).await?;

        let (stream_reader, stream_writer) = tokio::io::split(stream);

        //
        // let (message_sender, mut message_receiver) = mpsc::unbounded_channel();
        // この関数から脱出すると終了を知らせる為にチャンネルを作っておく
        // let (_disconnection_sender, disconnection_receiver) = oneshot::channel();
        // let mesg = ChannelBrokerMessage::NewConnection {
        //     connection_id: self.connection_id,
        //     sender: message_sender,
        //     disconnection: disconnection_receiver,
        // };

        // Reader
        let (read_bytes_sender, mut read_bytes_receiver) = mpsc::unbounded_channel();
        tokio::spawn(connection_reader(
            self.connection_id,
            stream_reader,
            read_bytes_sender,
        ));

        // Writer
        let (mut write_bytes_sender, write_bytes_receiver) = mpsc::unbounded_channel();
        tokio::spawn(connection_writer(
            self.connection_id,
            stream_writer,
            write_bytes_receiver,
        ));

        // let (session, mut results) = ServerSession::new();
        // //.map_err()
        // self.session = Some(session);

        // let remaining_bytes_results = self
        //     .session
        //     .as_mut()
        //     .unwrap()
        //     .handle_input(&recieve_bytes)?;
        // .map_err(|x| format!("Failed to handle input: {:?}", x))?;

        // results.extend(remaining_bytes_results);
        let (tx, rx) = mpsc::unbounded_channel();
        self.tx = Some(tx.clone());

        tokio::spawn(async move {
            loop {
                //
                // read_bytes_receiver.recv();
                tx.send();
                // write_bytes_sender.send(message)
            }
        });

        Ok(())
    }

    async fn handhsake_outgoing(
        &mut self,
        mut stream: TcpStream,
    ) -> Result<(TcpStream, BytesMut), ConnectionError> {
        let mut handshake = procedure::Handshake::new(
            self.connection_id,
            stream,
            BytesMut::new(),
            self.session_id,
            self.broadcast_id,
        );

        debug!(handshake = ?&handshake);

        // これは美しくない
        // DownStreamConnectionとかを返すのが良いか？
        let r = handshake.hello().await?;
        let (stream, buf) = handshake.raw_parts();

        Ok((stream, buf))
    }

    async fn spawn_task() {}

    pub async fn subscription(&mut self) -> Result<Atom, OutConnectionError> {
        let t = self.tx.unwrap();

        todo!()
    }
}

// クラスメソッド相当
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
        // if !send(&sender, buffer.freeze()) {
        //     break;
        // }

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
#[cfg(test)]
mod t {
    use super::*;

    #[test]
    fn test_outconnection() {
        let c1 = OutConnection::new(GnuId::new(), GnuId::new());
        let c2 = OutConnection::new(GnuId::new(), GnuId::new());
        let c3 = OutConnection::new(GnuId::new(), GnuId::new());

        assert_eq!(c1.connection_id, CONNECTION_ID_START);
        assert_eq!(c2.connection_id, CONNECTION_ID_START + 1);
        assert_eq!(c3.connection_id, CONNECTION_ID_START + 2);
    }
}
