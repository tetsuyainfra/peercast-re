use crate::pcp::{gnuid::GnuId, Atom, Id4};

/// Helloパケット群を作成する
///
/// example:
/// let packet = HelloBuilder::new()
///   .session_id(uid)
///   .broadcast_id(bid)
///   .port(port_no)       // Option
///   .ping(ping_port_no)  // Option
///   .build()
pub struct HelloBuilder {
    session_id: GnuId,
    port_no: Option<u16>,
    ping_no: Option<u16>,
    broadcast_id: GnuId,
}

impl HelloBuilder {
    pub fn new(session_id: GnuId, broadcast_id: GnuId) -> HelloBuilder {
        HelloBuilder {
            session_id: session_id,
            port_no: None,
            ping_no: None,
            broadcast_id: broadcast_id,
        }
    }

    pub fn port(mut self, port_no: u16) -> Self {
        self.port_no = Some(port_no);
        self
    }
    pub fn ping(mut self, ping_no: u16) -> Self {
        self.ping_no = Some(ping_no);
        self
    }

    pub fn build(&self) -> Atom {
        let mut vec = Vec::new();
        vec.push(Atom::AGENT.clone());
        vec.push(Atom::VERSION.clone());
        vec.push(Atom::Child(
            (Id4::PCP_HELO_SESSIONID, self.session_id).into(),
        ));
        vec.push(Atom::Child((Id4::PCP_HELO_BCID, self.broadcast_id).into()));

        if let Some(port_no) = self.port_no {
            vec.push(Atom::Child((Id4::PCP_HELO_PORT, port_no).into()))
        }
        if let Some(ping_to_port_no) = self.ping_no {
            vec.push(Atom::Child((Id4::PCP_HELO_PING, ping_to_port_no).into()))
        }

        Atom::Parent((Id4::PCP_HELO, vec).into())
    }
}
