use std::net::IpAddr;

use peercast_atom::{Atom, AtomMut, AtomTryDecode, AtomView, KindView};
use peercast_gnuid::GnuId;
use peercast_id4::Id4;

use crate::pcp::builder::{self, error::InfoParseError};

#[derive(Debug)]
pub struct OlehBuilder {
    session_id: GnuId,
    remote_ip: IpAddr,
    remote_port: u16,
}

impl OlehBuilder {
    ///
    /// @session_id: 送信元SessionID
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
        atoms.push(builder::AGENT.clone().into());
        atoms.push(builder::VERSION.clone().into());

        atoms.push((Id4::PCP_HELO_SESSIONID, self.session_id).into());

        //
        atoms.push((Id4::PCP_HELO_REMOTEIP, self.remote_ip).into());
        atoms.push((Id4::PCP_HELO_PORT, self.remote_port).into());

        (Id4::PCP_OLEH, atoms).into()
    }
}

#[derive(Debug, Default)]
pub struct OlehInfo {
    pub session_id: Option<GnuId>,
    pub remote_ip: Option<IpAddr>,
    pub agent: Option<Vec<u8>>,
    pub port: Option<u16>,
    pub version: Option<u32>,
}

impl TryFrom<&Atom> for OlehInfo {
    type Error = InfoParseError;

    fn try_from(atom: &Atom) -> Result<Self, Self::Error> {
        if atom.id() != Id4::PCP_OLEH {
            return Err(InfoParseError::TargetNotFound);
        }

        let pv = match atom.view() {
            KindView::Parent(pv) => pv,
            KindView::Child(_) => return Err(InfoParseError::TargetNotFound),
        };

        let mut oleh = OlehInfo::default();
        for child in pv.children() {
            if let KindView::Child(cv) = child {
                match cv.id() {
                    Id4::PCP_HELO_SESSIONID => {
                        oleh.session_id = cv.try_decode_gnuid();
                    }
                    Id4::PCP_HELO_REMOTEIP => {
                        oleh.remote_ip = cv.try_decode_ipaddr();
                    }
                    Id4::PCP_HELO_AGENT => {
                        // TODO: bytes::Bytesに変換する？
                        oleh.agent = cv.try_decode_vec()
                    }
                    Id4::PCP_HELO_PORT => {
                        oleh.port = cv.try_decode_u16();
                    }
                    Id4::PCP_HELO_VERSION => {
                        oleh.version = cv.try_decode_u32();
                    }
                    _ => {
                        tracing::debug!("skip atom({:?})", cv)
                    }
                }
            }
        }

        Ok(oleh)
    }
}

#[cfg(test)]
mod t {
    use super::*;

    #[test]
    fn test_builder() {
        let s = GnuId::new();
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        let port = 7144;

        let oleh_atom = OlehBuilder::new(s, ip, port).build();

        let x = OlehInfo::try_from(&oleh_atom.freeze().unwrap()).unwrap();
        assert_eq!(x.session_id, Some(s));
        assert_eq!(x.remote_ip, Some(ip));
        assert_eq!(x.port, Some(port));
    }

    #[test]
    fn test_builder_v6() {
        let s = GnuId::new();
        let ip: IpAddr = "2001:0DB8::0:0001".parse().unwrap();
        let port = 7145;

        let oleh_atom = OlehBuilder::new(s, ip, port).build();

        let x = OlehInfo::try_from(&oleh_atom.freeze().unwrap()).unwrap();
        assert_eq!(x.session_id, Some(s));
        assert_eq!(x.remote_ip, Some(ip));
        assert_eq!(x.port, Some(port));
    }
}
