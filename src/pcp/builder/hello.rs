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
        vec.push(Atom::Child((Id4::PCP_HELO_BCID, self.broadcast_id).into()));

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

#[derive(Debug)]
pub struct HeloInfo {
    ///
    pub session_id: GnuId,
    ///
    /// example: "PeerCast"
    pub agent: String,
    ///
    /// example: 1218
    pub version: u32,
    /// 自己申告してきたPort番号
    pub port: Option<u16>,
    /// 自己申告してきたPingして欲しい番号
    pub ping: Option<u16>,
    /// 配信時に使うID（認証に使う？）
    pub broadcast_id: Option<GnuId>,
}

impl HeloInfo {
    // #[rustfmt::skip]
    pub fn parse(atom: &Atom) -> Result<Self, AtomParseError> {
        if atom.id() != Id4::PCP_HELO {
            return Err(AtomParseError::IdError);
        }
        if atom.is_child() {
            return Err(AtomParseError::IdError);
        }

        let mut session_id = None;
        let mut agent = None;
        let mut version = None;
        let mut port = None;
        let mut ping = None;
        let mut broadcast_id = None;
        for a in atom.as_parent().childs() {
            trace!(atom = ?a);
            if a.is_parent() {
                return Err(AtomParseError::ValueError);
            }
            let a = a.as_child();
            match a.id() {
                Id4::PCP_HELO_AGENT => agent = Some(decode_string(a)?),
                Id4::PCP_HELO_VERSION => version = Some(decode_u32(a)?),
                Id4::PCP_HELO_SESSIONID => session_id = Some(decode_gnuid(a)?),
                Id4::PCP_HELO_PORT => port = Some(decode_u16(a)?),
                Id4::PCP_HELO_PING => ping = Some(decode_u16(a)?),
                Id4::PCP_HELO_BCID => broadcast_id = Some(decode_gnuid(a)?),
                _ => {
                    info!("UNKWON ATOM {:#?}", a);
                }
            }
        }

        trace!(?agent, ?version, ?session_id, ?port, ?ping, ?broadcast_id);

        let (agent, version, session_id) = match (agent, version, session_id) {
            (Some(a), Some(v), Some(s)) => (a, v, s),
            (_, _, _) => {
                return Err(AtomParseError::NotFoundValue);
            }
        };

        Ok(HeloInfo {
            session_id,
            agent,
            version,
            //
            port,
            ping,
            broadcast_id,
        })
    }
}
