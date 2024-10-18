// TODO: コネクションが切断された時の状態で、チャンネルを削除するか、5分程度削除を遅らせるか挙動を変える
// MEMO: 今は削除されるものとする(これはcreated_atが変わるため良く無い挙動である)

use std::sync::Arc;

use merge::vec;
use peercast_re::{
    pcp::{
        decode::{PcpBroadcast, PcpQuit},
        Atom, GnuId, Id4, PcpConnection, PcpConnectionReadHalf, PcpConnectionWriteHalf,
    },
    util::util_mpsc::mpsc_send,
    ConnectionId,
};
use tokio::sync::{
    broadcast,
    mpsc::{self, UnboundedReceiver, UnboundedSender},
};
use tower_http::trace;
use tracing::{debug, info, trace, warn};
use uuid::fmt::Braced;

use crate::{
    channel::TrackerChannelConfig,
    error::RootError,
    manager::{ConnectionMessage, RootManagerMessage},
};

/// Manager内操作
#[derive(Debug)]
enum SessionResult {
    RaisedEvent(ServerSessionEvent),
}

#[derive(Debug)]
enum ServerSessionEvent {
    PublishChannelRequested {
        atom: Arc<Atom>,
        broadcast: Arc<PcpBroadcast>,
    },
    PublishChannelFinished {},
    UpdateChannel {
        atom: Arc<Atom>,
    },
    FinishChannel {
        quit_atom: Arc<Atom>,
    },
}

pub enum State {
    Waiting,
    Running,
}

/// Session操作等を実行した後の動作
#[derive(Debug, PartialEq, Eq)]
pub enum ConnectionAction {
    None,
    Disconnect,
}

pub struct TrackerConnection {
    connection_id: ConnectionId,
    config: Arc<TrackerChannelConfig>,
    manager_sender: UnboundedSender<RootManagerMessage>,
    remote_broadcast_id: Arc<GnuId>,
    /// Handshake後、最初のパケット
    first_broadcast: Option<Arc<PcpBroadcast>>,
    state: State,
}

impl TrackerConnection {
    pub fn new(
        connection_id: ConnectionId,
        config: Arc<TrackerChannelConfig>,
        manager_sender: UnboundedSender<RootManagerMessage>,
        remote_broadcast_id: Arc<GnuId>,
        first_broadcast: Arc<PcpBroadcast>,
    ) -> Self {
        Self {
            connection_id,
            config,
            manager_sender,
            remote_broadcast_id,
            first_broadcast: Some(first_broadcast),
            state: State::Waiting,
        }
    }

    /// ConnectionManagerとの接続を開始する
    pub async fn start_connect_manager(
        mut self,
        connection: PcpConnection,
    ) -> Result<(), RootError> {
        info!(cid = ?connection.connection_id(), "START ConnectionManager");

        let remote_broadcast_id = self.remote_broadcast_id.clone();
        let remote_session_id = connection.remote_session_id.clone();
        let connection_id = connection.connection_id();
        let (message_sender, mut message_receiver) = mpsc::unbounded_channel();
        let (_disconnection_sender, disconnection_receiver) = mpsc::unbounded_channel();

        let message = RootManagerMessage::NewConnection {
            connection_id: connection.connection_id(),
            sender: message_sender,
            disconnection: disconnection_receiver,
        };
        if !mpsc_send(&self.manager_sender, message) {
            return Err(RootError::InitFailed);
        }

        let (read_half, write_half) = connection.split();
        let (reader_tx, mut reader_rx) = mpsc::unbounded_channel();
        let (mut writer_tx, writer_rx) = mpsc::unbounded_channel();

        let _ = tokio::spawn(read_routine(reader_tx, read_half));
        let _ = tokio::spawn(write_routine(writer_rx, write_half));

        // Publish Channel
        // Handshake後、最初のパケット(本当はstart_connect_managerに引数で与えたいけど複雑になるのでOptionで渡している)
        let first_broadcast = self.first_broadcast.take().unwrap();
        let message = RootManagerMessage::PublishChannel {
            connection_id: connection_id.clone(),
            session_id: remote_session_id,
            broadcast_id: remote_broadcast_id,
            first_broadcast,
        };
        if !mpsc_send(&self.manager_sender, message) {
            return Err(RootError::InitFailed);
        }

        // Start
        self.state = State::Running;

        let mut results = vec![];

        let _reason = loop {
            let action = self.handle_session_results(&mut results, &mut writer_tx)?;
            if action == ConnectionAction::Disconnect {
                break;
            }

            tokio::select! {
                // read_routineで受信したAtomはここで処理される
                atom = reader_rx.recv() => {
                    trace!(cid=?connection_id, atom=?atom, "CONNECTION ATOM COME");
                    match atom {
                        None => break,
                        Some(a) => { results = self.handle_arrived_atom(a)?; }
                    };
                },
                // Managerからメッセージが来たら処理
                manager_message = message_receiver.recv() => {
                    trace!(cid=?connection_id, action=?action, "CONNECTION MANAGER_MSG COME");
                    match manager_message {
                        None => break,
                        Some(message) => {
                            let (new_results, action) = self.handle_connection_message(message)?;
                            if action == ConnectionAction::Disconnect {
                                break;
                            }
                            results = new_results;
                        }
                    }
                }
            };
        };
        info!(cid = ?connection_id, "STOP ConnectionManager");

        Ok(())
    }

    fn handle_session_results(
        &mut self,
        results: &mut Vec<SessionResult>,
        writer_sender: &mut UnboundedSender<Atom>,
    ) -> Result<ConnectionAction, RootError> {
        if results.len() == 0 {
            return Ok(ConnectionAction::None);
        }

        let mut new_results = Vec::new();
        for result in results.drain(..) {
            match result {
                SessionResult::RaisedEvent(event) => {
                    let action = self.handle_raised_event(event, &mut new_results)?;
                    if action == ConnectionAction::Disconnect {
                        return Ok(ConnectionAction::Disconnect);
                    }
                }
            }
        }

        self.handle_session_results(&mut new_results, writer_sender)?;

        Ok(ConnectionAction::None)
    }

    fn handle_raised_event(
        &mut self,
        event: ServerSessionEvent,
        new_results: &mut Vec<SessionResult>,
    ) -> Result<ConnectionAction, RootError> {
        match event {
            ServerSessionEvent::PublishChannelRequested { atom, broadcast } => todo!(
                "今のところ、start_connect_managerの冒頭で処理しているので処理しないで良さそう"
            ),
            ServerSessionEvent::PublishChannelFinished {} => todo!("Finish処理"),
            ServerSessionEvent::UpdateChannel { atom } => {
                trace!(cid = ?self.connection_id,atom=?atom, "UpdateChannel");
                let broadcast = PcpBroadcast::parse(&atom)?;
                let message = RootManagerMessage::UpdateChannel {
                    connection_id: self.connection_id,
                    broadcast: Arc::new(broadcast),
                };
                if !mpsc_send(&self.manager_sender, message) {
                    return Err(RootError::InitFailed); // TODO: Change
                }
                Ok(ConnectionAction::None)
            }
            ServerSessionEvent::FinishChannel { quit_atom } => {
                trace!(cid = ?self.connection_id, quit_atom=?quit_atom, "FinishChannel");

                let quit = PcpQuit::parse(&quit_atom)?;
                let message = RootManagerMessage::FinishChannel {
                    connection_id: self.connection_id,
                    quit: Arc::new(quit),
                };
                if !mpsc_send(&self.manager_sender, message) {
                    return Err(RootError::InitFailed); // TODO: Change
                }
                Ok(ConnectionAction::Disconnect)
            }
        }
    }

    /// 通信相手から到着したAtomを処理する
    fn handle_arrived_atom(&self, atom: Atom) -> Result<Vec<SessionResult>, RootError> {
        match atom.id() {
            Id4::PCP_BCST => {
                trace!("{} ARRIVED_BCST {:?}", self.connection_id, atom);
                Ok(vec![SessionResult::RaisedEvent(
                    ServerSessionEvent::UpdateChannel {
                        atom: Arc::new(atom),
                    },
                )])
            }
            Id4::PCP_QUIT => {
                trace!("{} ARRIVED_QUIT {:?}", self.connection_id, atom);
                Ok(vec![SessionResult::RaisedEvent(
                    ServerSessionEvent::FinishChannel {
                        quit_atom: Arc::new(atom),
                    },
                )])
            }
            _ => {
                warn!("{} UNKNOWN ATOM: {:#?}", self.connection_id, atom);
                Ok(vec![])
            }
        }
    }

    /// RootManagerから到着したメッセージを処理する
    fn handle_connection_message(
        &mut self,
        msg: ConnectionMessage,
    ) -> Result<(Vec<SessionResult>, ConnectionAction), RootError> {
        match msg {
            ConnectionMessage::ConnectAccepted {} => todo!(),
            ConnectionMessage::ConnectRefused {} => todo!(),
            ConnectionMessage::Ok {} => todo!(),
            ConnectionMessage::FinishChannel {} => Ok((vec![], ConnectionAction::Disconnect)),
        }
    }
}

//------------------------------------------------------------------------------
// R/W Routine
async fn read_routine(
    mut tx: UnboundedSender<Atom>,
    mut read_half: PcpConnectionReadHalf,
) -> Result<(), std::io::Error> {
    let conn_id = read_half.connection_id();
    trace!(cid = ?conn_id, "START READ HALF");
    loop {
        let Ok(atom) = read_half.read_atom().await else {
            break;
        };
        // debug!("{conn_id} ARRIVED_ATOM {:?}", atom);
        mpsc_send(&mut tx, atom);
    }
    trace!(cid = ?conn_id, "STOP READ HALF");
    Ok(())
}
async fn write_routine(
    mut rx: UnboundedReceiver<Atom>,
    mut write_half: PcpConnectionWriteHalf,
) -> Result<(), std::io::Error> {
    let conn_id = write_half.connection_id();
    trace!(cid = ?conn_id, "START WRITE HALF");
    loop {
        let atom = rx.recv().await;
        match atom {
            None => break,
            Some(atom) => {
                debug!("{conn_id} WRITE_ATOM {}", atom);
                let _ = write_half.write_atom(atom).await;
            }
        };
    }
    trace!(cid = ?conn_id, "STOP WRITE HALF");
    Ok(())
}

#[cfg(test)]
mod tests {
    use peercast_re::pcp::GnuId;

    use crate::{channel::TrackerDetail, manager::RootManager};

    #[test]
    fn with_root_manager() {
        use RootManager;

        let channel_id = GnuId::new_arc();
        let (detail_sender, _) = tokio::sync::watch::channel(TrackerDetail {
            channel_info: todo!(),
            track_info: todo!(),
            created_at: todo!(),
            id: todo!(),
        });
        let x = RootManager::start(channel_id, detail_sender);
    }
}
