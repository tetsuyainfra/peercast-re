use peercast_atom::{Atom, AtomMut, AtomView, KindView};

use crate::{GnuId, Id4, pcp::builder::error::InfoParseError};

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

    pub fn build(self) -> AtomMut {
        let mut ping_childs = Vec::with_capacity(3);
        {
            let session_atom = AtomMut::from((Id4::PCP_HELO_SESSIONID, self.self_session_id));
            ping_childs.push(session_atom);

            if let Some(port) = self.port {
                let atom = AtomMut::from((Id4::PCP_HELO_PORT, port));
                ping_childs.push(atom)
            }
            if let Some(check_port) = self.port_check {
                let atom = AtomMut::from((Id4::PCP_HELO_PING, check_port));
                ping_childs.push(atom)
            }
        }
        let ping_atom = AtomMut::from((Id4::PCP_HELO, ping_childs));

        ping_atom.into()
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
        Self {
            session_id,
        }
    }

    pub fn build(self) -> AtomMut {
        let mut vec: Vec<AtomMut> = vec![(Id4::PCP_HELO_SESSIONID, self.session_id).into()];

        AtomMut::from((Id4::PCP_OLEH, vec))
    }
}

#[derive(Debug, Default)]
pub struct PongInfo {
    pub session_id: Option<GnuId>,
}

impl TryFrom<&Atom> for PongInfo {
    type Error = InfoParseError;

    fn try_from(atom: &Atom) -> Result<Self, Self::Error> {
        if atom.id() != Id4::PCP_OLEH {
            return Err(InfoParseError::TargetNotFound);
        }

        let pv = match atom.view() {
            KindView::Parent(pv) => pv,
            KindView::Child(_) => return Err(InfoParseError::TargetNotFound),
        };

        let info = PongInfo::default();
        for child in pv.children() {
            match child.id() {
                Id4::PCP_HELO_SESSIONID => {}
                _ => {}
            }
        }

        Ok(info)
    }
}
