use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    str,
};

use bytes::Buf;

use crate::pcp::{
    atom2::{Atom2Kind, AtomView},
    builder2::{self, InfoParseError},
    Atom2, AtomMut, GnuId, Id4,
};
use crate::prelude::*;

#[derive(Debug)]
pub struct OlehBuilder2 {
    session_id: GnuId,
    remote_ip: IpAddr,
    remote_port: u16,
}

impl OlehBuilder2 {
    ///
    /// @session_id: 送信側SessionID
    /// @remote_ip: 送信先のIP
    /// @remote_port: 送信先のポート(HELOで送ってきたPING_PORTをチェックした結果のポート) 0を入れても良い
    pub fn new(session_id: GnuId, remote_ip: IpAddr, remote_port: u16) -> Self {
        Self {
            session_id,
            remote_ip,
            remote_port,
        }
    }

    pub fn build(self) -> AtomMut {
        let mut atoms: Vec<AtomMut> = Vec::with_capacity(6);
        atoms.push(builder2::AGENT.clone().into());
        atoms.push(builder2::VERSION.clone().into());

        atoms.push((Id4::PCP_HELO_SESSIONID, self.session_id).into());

        //
        atoms.push((Id4::PCP_HELO_REMOTEIP, self.remote_ip).into());
        atoms.push((Id4::PCP_HELO_PING, self.remote_port).into());

        (Id4::PCP_OLEH, atoms).into()
    }
}

#[derive(Debug)]
pub struct OlehInfo {
    pub session_id: GnuId,
    pub remote_ip: Option<IpAddr>,
    pub agent: Option<String>,
    pub port: Option<u16>,
    pub version: Option<u32>,
}

impl TryFrom<&Atom2> for OlehInfo {
    type Error = InfoParseError;

    fn try_from(atom: &Atom2) -> Result<Self, Self::Error> {
        if atom.id() != Id4::PCP_OLEH {
            return Err(InfoParseError::TargetNotFound);
        }

        let pv = match atom.view() {
            Atom2Kind::Parent(pv) => pv,
            Atom2Kind::Child(_) => return Err(InfoParseError::TargetNotFound),
        };

        let mut session_id = None;
        let mut remote_ip = None;
        let mut agent = None;
        let mut port = None;
        let mut version = None;
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
            session_id: session_id.ok_or(InfoParseError::TargetNotFound)?,
            remote_ip,
            agent,
            port,
            version,
        })
    }
}
