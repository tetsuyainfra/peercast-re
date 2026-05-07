use peercast_atom::{Atom, AtomMut, AtomTryDecode, AtomView, KindView};
use peercast_gnuid::GnuId;
use peercast_id4::Id4;

use crate::pcp::builder::{self, error::InfoParseError};

pub struct HeloBuilder {
    agent: Option<String>,
    version: Option<u32>,
    session_id: GnuId,
    broadcast_id: Option<GnuId>,
    os_type: Option<i32>,
    port: Option<u16>,
    port_check: Option<u16>,
}

impl HeloBuilder {
    fn new(session_id: GnuId) -> Self {
        Self {
            agent: None,
            version: None,
            session_id,
            broadcast_id: None,
            os_type: None,
            port: None,
            port_check: None,
        }
    }
    pub fn broadcast_id(mut self, broadcast_id: GnuId) -> Self {
        self.broadcast_id = Some(broadcast_id);
        self
    }

    // 解放が確認されているポート番号
    // Confirmed Open Port Number
    pub fn port(mut self, open_confiremed_port_no: u16) -> Self {
        self.port = Some(open_confiremed_port_no);
        self
    }

    // 相手に解放してるか確認してほしいポート番号
    // I would like you to confirm Port Number as Open
    pub fn port_check(mut self, ping_port_no: u16) -> Self {
        self.port_check = Some(ping_port_no);
        self
    }

    pub fn build(&self) -> AtomMut {
        let mut vec = Vec::with_capacity(7);

        vec.push(builder::AGENT.clone().into());
        vec.push(builder::VERSION.clone().into());

        vec.push(AtomMut::from((Id4::PCP_HELO_SESSIONID, self.session_id)));

        if let Some(port) = self.port {
            vec.push(AtomMut::from((Id4::PCP_HELO_PORT, port)));
        }

        if let Some(port_check) = self.port_check {
            // portはu16だがAtomパケットではu32で送られる・・・
            vec.push(AtomMut::from((Id4::PCP_HELO_PING, port_check)))
        }

        if let Some(bcid) = self.broadcast_id {
            vec.push(AtomMut::from((Id4::PCP_HELO_BCID, bcid)));
        }

        AtomMut::from((Id4::PCP_HELO, vec))
    }
}

#[derive(Debug, Default)]
pub struct HeloInfoParts {
    pub agent: Option<Vec<u8>>,
    pub version: Option<u32>,
    pub os_type: Option<i32>,
    pub session_id: Option<GnuId>,
    pub broadcast_id: Option<GnuId>,
    /// 自分で解放しているポート番号
    pub port: Option<u16>,
    /// 解放しているか確認してほしいポート番号
    pub ping: Option<u16>,
}

impl TryFrom<&Atom> for HeloInfoParts {
    type Error = InfoParseError;

    fn try_from(atom: &Atom) -> Result<Self, Self::Error> {
        if atom.id() != Id4::PCP_HELO {
            return Err(InfoParseError::TargetNotFound);
        }

        let pv = match atom.view() {
            KindView::Parent(pv) => pv,
            KindView::Child(_) => return Err(InfoParseError::TargetNotFound),
        };

        let mut helo = HeloInfoParts::default();

        for child in pv.children() {
            let cv = match child {
                KindView::Parent(_) => continue,
                KindView::Child(cv) => cv,
            };
            match cv.id() {
                Id4::PCP_HELO_SESSIONID => helo.session_id = cv.try_decode_gnuid(),
                Id4::PCP_HELO_BCID => helo.broadcast_id = cv.try_decode_gnuid(),
                Id4::PCP_HELO_AGENT => helo.agent = cv.try_decode_vec(),
                Id4::PCP_HELO_VERSION => helo.version = cv.try_decode_u32(),
                Id4::PCP_HELO_OSTYPE => helo.os_type = cv.try_decode_i32(),
                // Id4::PCP_HELO_REMOTEIP => helo.remote_ip = cv.try_decode_u16(),
                Id4::PCP_HELO_PORT => helo.port = cv.try_decode_u16(),
                Id4::PCP_HELO_PING => helo.ping = cv.try_decode_u16(),
                // Id4::PCP_HELO_PONG => helo.pong = cv.try_decode_u16(),
                _ => {
                    continue;
                }
            }
        }

        Ok(helo)
    }
}

#[derive(Debug)]
pub struct HeloInfo<A> {
    pub parts: HeloInfoParts,
    pub atom: A,
}

impl<A> std::ops::Deref for HeloInfo<A> {
    type Target = HeloInfoParts;

    fn deref(&self) -> &Self::Target {
        &self.parts
    }
}

impl TryFrom<Atom> for HeloInfo<Atom> {
    type Error = InfoParseError;

    fn try_from(atom: Atom) -> Result<Self, Self::Error> {
        let parts = HeloInfoParts::try_from(&atom)?;
        Ok(HeloInfo {
            parts,
            atom,
        })
    }
}

impl<'a> TryFrom<&'a Atom> for HeloInfo<&'a Atom> {
    type Error = InfoParseError;

    fn try_from(atom: &'a Atom) -> Result<Self, Self::Error> {
        let parts = HeloInfoParts::try_from(atom)?;
        Ok(HeloInfo {
            parts,
            atom,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_helo_builder() {
        let session_id = GnuId::new();
        let bcid = GnuId::new();
        let helo_mut = HeloBuilder::new(session_id).broadcast_id(bcid).port(1).port_check(2).build();
        assert_eq!(helo_mut.id(), Id4::PCP_HELO);
        let helo: Atom = helo_mut.freeze().unwrap();
        assert_eq!(helo.id(), Id4::PCP_HELO);
        assert_eq!(helo.length(), 6);
    }
}
