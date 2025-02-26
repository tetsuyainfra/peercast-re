use tracing::{info, trace};

use crate::{
    error::AtomParseError,
    pcp::{
        decode::{decode_gnuid, decode_i32, decode_string, decode_u16, decode_u32},
        Atom, GnuId, Id4,
    },
};

#[derive(Debug)]
pub struct PcpHelo {
    /// 申告してきたホストのSessionId
    pub session_id: GnuId,
    /// example: "PeerCast"
    pub agent: String,
    /// example: 1218
    pub version: u32,
    /// 自己申告してきたPort番号
    pub port: Option<u16>,
    /// 自己申告してきたPingして欲しい番号
    pub ping: Option<u16>,
    /// 配信時に使うID（認証に使う？）
    pub broadcast_id: Option<GnuId>,

    // BANされている時に値が入る
    pub disable: Option<i32>,
}

impl PcpHelo {
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
        let mut disable = None;
        for a in atom.as_parent().childs() {
            // trace!(atom = ?a);
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
                Id4::PCP_HELO_DISABLE => disable = Some(decode_i32(a)?),
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

        Ok(PcpHelo {
            session_id,
            agent,
            version,
            //
            port,
            ping,
            broadcast_id,
            disable,
        })
    }
}
