use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    str,
};

use bytes::Buf;

use crate::pcp::{
    atom2::{Atom2Kind, AtomView},
    builder2::ParseError,
    Atom2, GnuId, Id4,
};
use crate::prelude::*;

#[derive(Debug)]
pub struct OlehInfo {
    pub session_id: GnuId,
    pub remote_ip: Option<IpAddr>,
    pub agent: Option<String>,
    pub port: Option<u16>,
    pub port_check: Option<u16>,
    pub version: Option<u32>,
}

impl TryFrom<Atom2> for OlehInfo {
    type Error = ParseError;

    fn try_from(atom: Atom2) -> Result<Self, Self::Error> {
        if atom.id() != Id4::PCP_OLEH {
            return Err(ParseError::TargetNotFound);
        }

        let pv = match atom.view() {
            Atom2Kind::Parent(pv) => pv,
            Atom2Kind::Child(_) => return Err(ParseError::TargetNotFound),
        };

        let (mut session_id, mut remote_ip, mut agent, mut port, mut port_check, mut version) =
            (None, None, None, None, None, None);

        for child in pv.children() {
            if let Atom2Kind::Child(cv) = child {
                match cv.id() {
                    Id4::PCP_HELO_SESSIONID => {
                        if cv.payload().len() == 16 {
                            session_id = Some(GnuId::from(cv.payload().get_u128()));
                        }
                    }
                    Id4::PCP_HELO_REMOTEIP => {
                        let payload = cv.payload();
                        if payload.len() == 4 {
                            remote_ip = Some(IpAddr::V4(Ipv4Addr::from(cv.payload().get_u32_le())));
                        } else if payload.len() == 16 {
                            remote_ip = Some(IpAddr::V6(Ipv6Addr::from(cv.payload().get_u128())));
                        }
                    }
                    Id4::PCP_HELO_AGENT => {
                        if let Ok(s) = str::from_utf8(cv.payload()) {
                            agent = Some(s.to_string());
                        }
                    }
                    Id4::PCP_HELO_PORT => {
                        if cv.payload().len() >= 2 {
                            port = Some(cv.payload().get_u16_le());
                        }
                    }
                    Id4::PCP_HELO_PING => {
                        if cv.payload().len() >= 2 {
                            port_check = Some(cv.payload().get_u16_le());
                        }
                    }
                    Id4::PCP_HELO_VERSION => {
                        if cv.payload().len() >= 4 {
                            version = Some(cv.payload().get_u32_le());
                        }
                    }
                    _ => {
                        debug!("skip atom({:?})", cv)
                    }
                }
            }
        }

        Ok(Self {
            session_id: session_id.ok_or(ParseError::TargetNotFound)?,
            remote_ip,
            agent,
            port,
            port_check,
            version,
        })
    }
}
