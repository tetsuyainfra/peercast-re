use std::{
    collections::{HashMap, VecDeque},
    net::SocketAddr,
};

use tower::util::error::optional::None;

use crate::pcp::{builder::HostInfo, session, GnuId};

// Handshakeをretryできる最大回数
const MAX_HANDSHAKE_RETRY: i8 = 3;

pub struct Node {
    node_data: NodeData,
    retries: i8,
}

pub enum NodeType {
    Server,
    Peer,
}

enum NodeData {
    Server {
        addr: SocketAddr,
        session_id: Option<GnuId>,
    },
    Peer {
        host_info: HostInfo,
    },
}

impl Node {
    fn server(addr: SocketAddr, session_id: Option<GnuId>) -> Self {
        Node {
            node_data: NodeData::Server { addr, session_id },
            retries: 0,
        }
    }
    fn peer(host_info: HostInfo) -> Self {
        Node {
            node_data: NodeData::Peer { host_info },
            retries: 0,
        }
    }

    // ---- method ----
    fn addr(&self) {}

    fn update_server_session_id(&mut self, new_session_id: Option<GnuId>) {
        match &mut self.node_data {
            NodeData::Server { addr, session_id } => *session_id = new_session_id,
            NodeData::Peer { host_info } => panic!("this is error"),
        };
    }

    fn retries(&self) {}
    fn retry_countup(&mut self) {
        self.retries += 1;
    }
    fn retry_reset(&mut self) {
        self.retries = 0;
    }
}

pub struct NodePool {
    server: SocketAddr,
    hosts: VecDeque<Node>,
    faild_hosts: HashMap<GnuId, HostInfo>,
}

impl NodePool {
    fn next_candidate(&mut self) -> Option<Node> {
        self.hosts.pop_front()
    }

    fn stock(&mut self, node: Node) {
        if node.retries < MAX_HANDSHAKE_RETRY {
            self.hosts.push_back(node)
        } else {
        }
    }
}

#[derive(Debug, Clone)]
pub(super) enum HostCandidate {
    Server {
        // MEMO: 最初に接続するPeercastはsession_idが分らないので0されてる物を使うことになる
        // また、各接続で帰ってきたOlehパケットのSessionIDで毎回上書きすることになる。ハンドシェイクの時だけだし負荷にはならないだろうがメモとして残す
        session_id: GnuId,
        addr: SocketAddr,
        retries: i8,
    },
    Peer {
        session_id: GnuId,
        addr: SocketAddr,
        retries: i8,
    },
}

impl HostCandidate {
    pub fn server(session_id: GnuId, addr: SocketAddr) -> Self {
        Self::Server {
            session_id,
            addr: addr,
            retries: 0,
        }
    }

    pub fn peer(session_id: GnuId, addr: SocketAddr) -> Self {
        Self::Peer {
            session_id,
            addr: addr,
            retries: 0,
        }
    }
    // ---- method ----

    pub fn session_id(&self) -> GnuId {
        match self {
            HostCandidate::Server { session_id, .. } => *session_id,
            HostCandidate::Peer { session_id, .. } => *session_id,
        }
    }
    pub fn set_session_id(&mut self, new_session_id: GnuId) {
        match self {
            HostCandidate::Server { session_id, .. } => *session_id = new_session_id,
            HostCandidate::Peer { session_id, .. } => *session_id = new_session_id,
        };
    }

    pub fn addr(&self) -> SocketAddr {
        match self {
            HostCandidate::Server { addr, .. } => *addr,
            HostCandidate::Peer { addr, .. } => *addr,
        }
    }

    pub fn retries(&self) -> i8 {
        match self {
            HostCandidate::Server { retries, .. } => *retries,
            HostCandidate::Peer { retries, .. } => *retries,
        }
    }

    pub fn add_retry(&mut self) {
        match self {
            HostCandidate::Server { retries, .. } => *retries = *retries + 1,
            HostCandidate::Peer { retries, .. } => *retries = *retries + 1,
        }
    }
    pub fn reset_retry(&mut self) {
        match self {
            HostCandidate::Server { retries, .. } => *retries = 0,
            HostCandidate::Peer { retries, .. } => *retries = 0,
        }
    }
}
