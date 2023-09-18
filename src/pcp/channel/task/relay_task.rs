use std::{collections::VecDeque, net::SocketAddr, time::Duration};

use bytes::{Bytes, BytesMut};
use futures_util::FutureExt;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    net::TcpStream,
    sync::{mpsc, watch},
    task::JoinHandle,
};
use tracing::{debug, error, info, warn};

use crate::{
    error::{ConnectionError, HandshakeError},
    pcp::{
        builder::OlehInfo,
        channel::{node_pool::HostCandidate, ChannelBrokerMessage},
        procedure::{BothHandshake, HandshakeReturn},
        session::{Session, SessionConfig, SessionEvent, SessionResult},
        Atom, ChannelInfo, GnuId, TrackInfo,
    },
    util::util_mpsc::send,
    ConnectionId,
};

use super::{SourceTaskConfig, SourceTaskTrait, TaskStatus};

////////////////////////////////////////////////////////////////////////////////
//  Relation Struct
//

pub struct RelayTaskConfig {
    pub session_id: GnuId,
    pub addr: SocketAddr,
}
impl From<RelayTaskConfig> for SourceTaskConfig {
    fn from(value: RelayTaskConfig) -> Self {
        SourceTaskConfig::Relay(value)
    }
}

////////////////////////////////////////////////////////////////////////////////
// ChannelTaskWorker
//

#[derive(Debug)]
pub struct RelayTask {
    worker_status: watch::Receiver<TaskState>,
    worker_handle: Option<JoinHandle<Result<(), ConnectionError>>>,
    // worker_shutdown: Shutdown,
}

impl RelayTask {
    pub(crate) fn new(
        broadcast_id: GnuId,
        connection_id: ConnectionId,
        config: RelayTaskConfig,
        broker_sender: mpsc::UnboundedSender<ChannelBrokerMessage>,
    ) -> Self {
        let RelayTaskConfig { session_id, addr } = config;
        let (status_tx, status_rx) = watch::channel(TaskState::Init);

        let worker = ChannelTaskWoker::new(
            broadcast_id,
            connection_id,
            session_id,
            addr,
            //
            broker_sender,
            //
            status_tx,
            // worker_shutdown.reciever(),
        );
        let worker_handle = tokio::spawn(async {
            // let connection_id = worker.connection_id.clone();
            let result = worker.start().await;
            // info!("{:?}: worker finished {:?}", connection_id, &result);
            result
        });

        RelayTask {
            worker_status: status_rx,
            worker_handle: Some(worker_handle),
            // worker_shutdown,
        }
    }

    pub async fn status_changed(&mut self) -> Result<TaskStatus, watch::error::RecvError> {
        self.worker_status.changed().await?;
        Ok(self.status())
    }

    //
    pub async fn wait(&mut self) {
        self.worker_handle.take().unwrap().await;
    }

    pub async fn stop(mut self) {
        debug!("stop");
        // self.worker_shutdown.shutdown().await;
        debug!("stop end");
    }

    pub fn blocking_shutdown(mut self) {
        // これはBlockingだけのコンテキストで呼ばないとだめ
        debug!("blocking_shutdown");
        // self.worker_shutdown.blocking_shutdown();
        debug!("blocking_shutdown end");
    }
}

#[async_trait::async_trait]
impl SourceTaskTrait for RelayTask {
    fn connect(&self, config: SourceTaskConfig) {
        todo!()
    }

    fn retry(&self) {
        todo!()
    }

    fn update_info(&self, info: ChannelInfo) {
        todo!()
    }

    fn update_track(&self, info: TrackInfo) {
        todo!()
    }

    fn status(&self) -> TaskStatus {
        match *self.worker_status.borrow() {
            TaskState::Init => TaskStatus::Idle,
            TaskState::Handshake => TaskStatus::Running,
            TaskState::Recieving => TaskStatus::Running,
            TaskState::Finish => TaskStatus::Stopped,
            TaskState::Error => TaskStatus::Error,
        }
    }

    async fn status_changed(&mut self) -> Result<(), watch::error::RecvError> {
        self.worker_status.changed().await
    }

    async fn stop(&self) {
        todo!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum TaskState {
    Init,
    Handshake,
    Recieving,
    Finish,
    Error,
}

////////////////////////////////////////////////////////////////////////////////
// ChannelTaskWorker
//
#[derive(Debug)]
struct ChannelTaskWoker {
    broadcast_id: GnuId,
    connection_id: ConnectionId,
    //
    session_id: GnuId,
    //
    root_addr: SocketAddr, // rootって言うのが正しいのかなぁ・・・
    target_hosts: VecDeque<HostCandidate>,
    failed_hosts: VecDeque<HostCandidate>,
    //
    broker_sender: mpsc::UnboundedSender<ChannelBrokerMessage>,
    //
    status_tx: watch::Sender<TaskState>,
    // shutdown: ShutdownRecvier,
    //
    session: Session,
}

const TCP_CONNECT_TIMEOUT: Duration = Duration::from_millis(5000);

impl ChannelTaskWoker {
    fn new(
        broadcast_id: GnuId,
        connection_id: ConnectionId,
        //
        session_id: GnuId,
        addr: SocketAddr,
        //
        broker_sender: mpsc::UnboundedSender<ChannelBrokerMessage>,
        //
        status_tx: watch::Sender<TaskState>,
        // shutdown: ShutdownRecvier,
    ) -> Self {
        Self {
            broadcast_id,
            connection_id,
            //
            session_id,
            root_addr: addr,
            target_hosts: VecDeque::from([HostCandidate::server(GnuId::from(0), addr)]),
            failed_hosts: VecDeque::new(),
            //
            broker_sender,
            //
            status_tx,
            // shutdown,
            session: Session::new(SessionConfig::new()),
        }
    }

    async fn start(mut self) -> Result<(), ConnectionError> {
        self.status_tx.send(TaskState::Handshake);

        // Peerに接続する
        let (stream, read_buf, oleh) = self.connect_to_peer().await?;

        let (stream_reader, stream_writer) = tokio::io::split(stream);

        let (read_bytes_sender, mut read_bytes_receiver) = mpsc::unbounded_channel();
        tokio::spawn(connection_reader(1, stream_reader, read_bytes_sender));

        let (mut write_bytes_sender, write_bytes_receiver) = mpsc::unbounded_channel();
        tokio::spawn(connection_writer(1, stream_writer, write_bytes_receiver));

        // Brokerに通知する
        let (broker_sender, mut broker_reciever) = mpsc::unbounded_channel();
        let (disconnection_sender, disconnection_reader) = mpsc::unbounded_channel();
        let message = ChannelBrokerMessage::NewConnection {
            connection_id: ConnectionId::new(),
            sender: broker_sender,
            disconnection: disconnection_reader,
        };
        if !send(&self.broker_sender, message) {
            return Ok(());
        }

        let mut results: Vec<SessionResult> = vec![];
        let mut remaining_results = self
            .session
            .handle_input(&read_buf[..])
            .map_err(|e| format!("Error occuerd"))
            .unwrap();

        results.extend(remaining_results);

        self.status_tx.send(TaskState::Recieving);

        loop {
            // リモートにデータを送る
            let reaction = self
                .handle_session_results(&mut results, &mut write_bytes_sender)
                .map_err(|x| format!("error"))
                .unwrap();
            if reaction == ConnectionReaction::Disconnect {
                info!("ConnectionReaction::Disconnect");
                break;
            }

            tokio::select! {
                // リモートからデータが来たデータをAtomにパースする
                // 来たデータはresultsに格納されるので、次のループ初回(handle_session_results)でブローカーに送って処理を決定する
                // ※broker_senderはselfのメンバー
                message = read_bytes_receiver.recv() => {
                    match message {
                        None => break,
                        Some(bytes) => {
                            results = self.session.handle_input(&bytes).map_err(|x| format!("error in message arrive")).unwrap();
                        }
                    }
                },
                // ブローカーからメッセージが着たら処理する
                manager_message = broker_reciever.recv() => {
                    // trace!("{}: Broker Message Arrived {:?}", &self.connection_id, &manager_message);
                    match manager_message {
                        None => break,
                    //     Some(message) => {
                    //         let (new_results, action) = self.handle_connection_message(message)?;
                    //         match action {
                    //             ConnectionAction::Disconnect => break,
                    //             _ => (),
                    //         };

                    //         results = new_results;
                    //     }
                        _ => {}
                    }
                }
            }
        }

        drop(disconnection_sender);
        self.status_tx.send(TaskState::Finish);
        info!("BID {:.7}: ChannelTaskWorker shutdown", self.broadcast_id);
        Ok(())
    }

    async fn connect_to_peer(&mut self) -> Result<(TcpStream, BytesMut, OlehInfo), HandshakeError> {
        info!(connection_id = ?self.connection_id, "connect_to_peer() start");
        const MAX_RETRY: i8 = 3;

        fn select_host(hosts: &mut VecDeque<HostCandidate>) -> Option<HostCandidate> {
            let target = hosts.pop_front();
            target
        }

        fn push_buck_or_failed(
            mut target: HostCandidate,
            target_hosts: &mut VecDeque<HostCandidate>,
            failed_hosts: &mut VecDeque<HostCandidate>,
        ) {
            target.add_retry();
            if target.retries() < MAX_RETRY {
                target_hosts.push_back(target)
            } else {
                failed_hosts.push_back(target)
            }
        }

        loop {
            let Some(mut target) = select_host(&mut self.target_hosts) else {
                return Err(HandshakeError::ServerNotFound);
            };

            let Ok(stream_result): Result<
                Result<TcpStream, std::io::Error>,
                tokio::time::error::Elapsed,
            > = tokio::time::timeout(TCP_CONNECT_TIMEOUT, TcpStream::connect(target.addr())).await
            else {
                error!(connection_id = ?self.connection_id, "timeout TcpStream::connect({:?})", target.addr());
                push_buck_or_failed(target, &mut self.target_hosts, &mut self.failed_hosts);
                continue;
            };
            let stream = TcpStream::connect(target.addr()).await?;
            info!(connection_id = ?self.connection_id, "TcpStream::connect({:?})", target);

            let Ok(handshake_result) = BothHandshake::new(
                self.connection_id,
                stream,
                self.session_id,
                self.broadcast_id,
            )
            .outgoing()
            .await
            else {
                error!(connection_id = ?self.connection_id, "timeout TcpStream::connect({:?})", target.addr());
                push_buck_or_failed(target, &mut self.target_hosts, &mut self.failed_hosts);
                continue;
            };

            match handshake_result {
                // 接続先が満杯だった
                HandshakeReturn::NextHost { oleh, hosts, quit } => {
                    target.set_session_id(oleh.session_id);
                    for host in hosts.into_iter() {
                        // FIXME: targetは自分自身のはずなので含まれていないはずだけど念のため確認する？
                        let exist_target_hosts = self
                            .target_hosts
                            .iter()
                            .find(|h| h.session_id() == host.session_id);
                        let exist_failed_hosts = self
                            .failed_hosts
                            .iter()
                            .find(|h| h.session_id() == host.session_id);
                        match (exist_target_hosts, exist_failed_hosts) {
                            // 登録する
                            (None, None) => {
                                let new_peer = HostCandidate::peer(
                                    host.session_id,
                                    host.global_address.unwrap(), // 必ず存在するはず
                                );
                                self.target_hosts.push_back(new_peer);
                            }
                            // すでに試してエラーになっている
                            (None, Some(_)) => {}
                            // すでにHostリストにある
                            (Some(_), None) => {}
                            (Some(_), Some(_)) => panic!("両方にあるのはおかしい"),
                        };
                    }
                    // 使った接続を戻す
                    target.add_retry(); // エラーにする必要ある？
                    push_buck_or_failed(target, &mut self.target_hosts, &mut self.failed_hosts);
                    continue;
                }
                // 接続先はChannel持ってなかった
                HandshakeReturn::ChannelNotFound => match target {
                    HostCandidate::Server { .. } => return Err(HandshakeError::ServerNotFound),
                    HostCandidate::Peer { .. } => drop(target),
                },
                HandshakeReturn::Success {
                    stream,
                    read_buf,
                    oleh,
                } => {
                    target.set_session_id(oleh.session_id);
                    target.reset_retry();
                    // エラー起きたら再接続するけど、一番最初にいると延々とハンドシェイク→エラーが起きかねないのでこうする。
                    // 本当は最初に入れてエラーを頻発させた方が良いかもしれない
                    push_buck_or_failed(target, &mut self.target_hosts, &mut self.failed_hosts);
                    return Ok((stream, read_buf, oleh));
                }
            }
        }
    }

    fn handle_session_results(
        &mut self,
        results: &mut Vec<SessionResult>,
        byte_writer: &mut mpsc::UnboundedSender<Atom>,
    ) -> Result<ConnectionReaction, Box<dyn std::error::Error + Sync + Send>> {
        if results.len() == 0 {
            return Ok(ConnectionReaction::None);
        }

        let mut new_results = Vec::new();
        for result in results.drain(..) {
            match result {
                // リモートへ送る
                SessionResult::OutboundResponse(a) => {
                    //
                }

                // イベントが発生した場合
                SessionResult::RaisedEvent(event) => {
                    // trace!("RaisedEvent here, {:#?}", &event);
                    let action = self.handle_raised_event(event, &mut new_results)?;
                    if action == ConnectionReaction::Disconnect {
                        return Ok(ConnectionReaction::Disconnect);
                    }
                }
                SessionResult::Unknown(a) => {
                    warn!("unknown atom arrived {:?}[{}]", a.id(), a.len());
                }
            }
        }
        self.handle_session_results(&mut new_results, byte_writer)?;

        return Ok(ConnectionReaction::None);
    }
    fn handle_raised_event(
        &mut self,
        event: SessionEvent,
        new_results: &mut Vec<SessionResult>,
    ) -> Result<ConnectionReaction, Box<dyn std::error::Error + Sync + Send>> {
        match event {
            SessionEvent::ArrivedHeadData {
                atom,
                head_data,
                info,
                track,
                pos,
            } => {
                //
                let messages = ChannelBrokerMessage::AtomHeadData {
                    atom,
                    payload: head_data,
                    pos,
                    info,
                    track,
                };
                send(&self.broker_sender, messages);
                Ok(ConnectionReaction::None)
            }
            //
            SessionEvent::ArrivedData {
                atom,
                data,
                pos,
                continuation,
            } => {
                //
                let messages = ChannelBrokerMessage::AtomData {
                    atom,
                    data,
                    pos,
                    continuation,
                };
                send(&self.broker_sender, messages);
                Ok(ConnectionReaction::None)
            }
        }
    }

    fn stop() {}
}

#[derive(Debug, PartialEq)]
enum ConnectionReaction {
    None,
    Disconnect,
}

async fn connection_reader(
    connection_id: u64,
    mut stream: ReadHalf<TcpStream>,
    manager: mpsc::UnboundedSender<Bytes>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut buffer = BytesMut::with_capacity(4096);

    loop {
        let bytes_read = stream.read_buf(&mut buffer).await?;
        if bytes_read == 0 {
            break;
        }

        let bytes = buffer.split_off(bytes_read);
        if !send(&manager, buffer.freeze()) {
            break;
        }

        buffer = bytes;
    }

    println!("Connection {}: Reader disconnected", connection_id);
    Ok(())
}

async fn connection_writer(
    connection_id: u64,
    mut stream: WriteHalf<TcpStream>,
    mut packets_to_send: mpsc::UnboundedReceiver<Atom>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    const BACKLOG_THRESHOLD: usize = 100;
    let mut send_queue = VecDeque::new();

    loop {
        let packet = packets_to_send.recv().await;
        if packet.is_none() {
            break; // connection closed
        }

        let packet = packet.unwrap();

        // Since RTMP is TCP based, if bandwidth is low between the server and the client then
        // we will end up backlogging the mpsc receiver.  However, mpsc does not have a good
        // way to know how many items are pending.  So we need to receive all pending packets
        // in a non-blocking manner, put them in a queue, and if the queue is too large ignore
        // optional packets.
        send_queue.push_back(packet);
        while let Some(Some(packet)) = packets_to_send.recv().now_or_never() {
            send_queue.push_back(packet);
        }

        // let mut send_optional_packets = true;
        // if send_queue.len() > BACKLOG_THRESHOLD {
        //     println!(
        //         "Connection {}: Too many pending packets, dropping optional ones",
        //         connection_id
        //     );
        //     send_optional_packets = false;
        // }

        // for packet in send_queue.drain(..) {
        //     if send_optional_packets || !packet.can_be_dropped {
        //         stream.write_all(packet.bytes.as_ref()).await?;
        //     }
        // }
        let mut send_buf = BytesMut::with_capacity(4096);
        for atom in send_queue.drain(..) {
            atom.write_bytes(&mut send_buf);
        }
        stream.write_all_buf(&mut send_buf);
    }

    println!("Connection {}: Writer disconnected", connection_id);
    Ok(())
}

#[cfg(test)]
mod t {
    use crate::pcp::channel::broker::ChannelBroker;

    use super::super::SourceTaskConfig;
    use super::*;

    #[ignore = "Not yet implement"]
    #[crate::test]
    async fn test() {
        let session_id = GnuId::new();
        let broadcast_id = GnuId::new();
        let addr = "127.0.0.1:3000".parse().unwrap();
        let broker_task = ChannelBroker::new(broadcast_id, Default::default(), Default::default());
        let config = RelayTaskConfig {
            session_id: session_id,
            addr: addr,
        };
        let task = RelayTask::new(
            broadcast_id,
            ConnectionId::new(),
            config,
            broker_task.sender(),
        );

        loop {
            tokio::task::yield_now().await;
        }
    }
}
