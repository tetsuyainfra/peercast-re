use std::pin::Pin;

use bytes::Buf;
use tracing::{info, trace};

use crate::{
    error::AtomParseError,
    pcp::{
        atom::decode::{decode_gnuid, decode_string, decode_u16, decode_u32},
        builder::broadcast,
        gnuid::GnuId,
        session, Atom, Id4,
    },
};

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
    broadcast_id: Option<GnuId>,
}

impl HelloBuilder {
    pub fn new(session_id: GnuId, broadcast_id: Option<GnuId>) -> HelloBuilder {
        HelloBuilder {
            session_id: session_id,
            port_no: None,
            ping_no: None,
            broadcast_id: broadcast_id,
        }
    }

    // 解放が確認されているポート番号
    // Confirmed Open Port Number
    pub fn port(mut self, open_confiremed_port_no: u16) -> Self {
        self.port_no = Some(open_confiremed_port_no);
        self
    }

    // 相手に解放してるか確認してほしいポート番号
    // I would like you to confirm Port Number as Open
    pub fn ping(mut self, ping_port_no: u16) -> Self {
        self.ping_no = Some(ping_port_no);
        self
    }

    pub fn build(&self) -> Atom {
        let mut vec = Vec::new();
        vec.push(Atom::AGENT.clone());
        vec.push(Atom::VERSION.clone());
        vec.push(Atom::Child(
            (Id4::PCP_HELO_SESSIONID, self.session_id).into(),
        ));
        if self.broadcast_id.is_some() {
            vec.push(Atom::Child(
                (Id4::PCP_HELO_BCID, self.broadcast_id.unwrap()).into(),
            ));
        }

        if let Some(port_no) = self.port_no {
            // portはu16だがAtomパケットではu32で送られる・・・
            vec.push(Atom::Child((Id4::PCP_HELO_PORT, port_no).into()))
        }
        if let Some(ping_to_port_no) = self.ping_no {
            // portはu16だがAtomパケットではu32で送られる・・・
            vec.push(Atom::Child((Id4::PCP_HELO_PING, ping_to_port_no).into()))
        }

        Atom::Parent((Id4::PCP_HELO, vec).into())
    }
}
