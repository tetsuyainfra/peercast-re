use std::collections::{vec_deque, VecDeque};

use bytes::Buf;

use crate::{
    error::AtomParseError,
    pcp::{atom::decode::decode_gnuid, Atom, GnuId, Id4},
};

////////////////////////////////////////////////////////////////////////////////
//  Ping
//
/// Pingする時に使うAtomを生成する(最初のPCP_CONNECTを含む)
#[derive(Debug)]
pub struct PingBuilder {
    self_session_id: GnuId,
    port: Option<u16>,
    port_check: Option<u16>,
}

impl PingBuilder {
    pub fn new(self_session_id: GnuId) -> Self {
        Self {
            self_session_id,
            port: None,
            port_check: None,
        }
    }

    pub fn port(mut self, port: Option<u16>) -> Self {
        self.port = port;
        self
    }

    pub fn port_check(mut self, port_check: Option<u16>) -> Self {
        self.port_check = port_check;
        self
    }

    pub fn build(self) -> VecDeque<Atom> {
        let magic_atom = Atom::Child((Id4::PCP_CONNECT, 1_u32).into());

        let session_atom = Atom::Child((Id4::PCP_HELO_SESSIONID, self.self_session_id).into());
        let mut atoms = Vec::with_capacity(3);
        atoms.push(session_atom);

        if let Some(port) = self.port {
            let atom = Atom::Child((Id4::PCP_HELO_PORT, port).into());
            atoms.push(atom)
        }
        if let Some(check_port) = self.port_check {
            let atom = Atom::Child((Id4::PCP_HELO_PING, check_port).into());
            atoms.push(atom)
        }

        let ping_atom = Atom::Parent((Id4::PCP_HELO, atoms).into());

        VecDeque::from([magic_atom, ping_atom])
    }
}

////////////////////////////////////////////////////////////////////////////////
//  Pong
//

#[derive(Debug)]
pub struct PongBuilder {
    session_id: GnuId,
}

impl PongBuilder {
    pub fn new(session_id: GnuId) -> Self {
        Self { session_id }
    }

    pub fn build(self) -> Atom {
        let mut vec: Vec<Atom> = Vec::new();
        vec.push(Atom::Child(
            (Id4::PCP_HELO_SESSIONID, self.session_id).into(),
        ));

        Atom::Parent((Id4::PCP_OLEH, vec).into())
    }
}
