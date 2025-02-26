use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    str,
};

use bytes::Buf;
use tracing::debug;

use crate::pcp::{Atom, GnuId, Id4};

pub struct OlehBuilder {
    // agent: String,
    session_id: GnuId,
    remote_ip: IpAddr,
    remote_port: u16,
}

#[allow(dead_code)]
impl OlehBuilder {
    //! Olehパケットを作成する
    //! session_id : このマシンを同定する唯一のID
    //! remote_ip : 返信先マシンのポート番号(Heloを送ってきたアドレス)
    //! remote_port : 返信先マシンのポート番号、ポート開放チェックに失敗したらNone(Heloで送られてきたPINGポート)
    pub fn new(session_id: GnuId, remote_ip: IpAddr, remote_port: u16) -> Self {
        Self {
            session_id,
            remote_ip,
            remote_port,
        }
    }

    pub fn build(&self) -> Atom {
        let mut vec = Vec::new();
        vec.push(Atom::AGENT.clone());
        vec.push(Atom::Child(
            (Id4::PCP_HELO_SESSIONID, self.session_id).into(),
        ));
        vec.push(Atom::VERSION.clone());
        vec.push(Atom::Child((Id4::PCP_HELO_REMOTEIP, self.remote_ip).into()));
        vec.push(Atom::Child((Id4::PCP_HELO_PORT, self.remote_port).into()));

        Atom::Parent((Id4::PCP_OLEH, vec).into())
    }
}

#[derive(Debug)]
pub struct OlehInfo {
    pub remote_ip: Option<IpAddr>,
    pub agent: String,
    pub session_id: GnuId,
    pub port: u16,
    pub version: u32,
}

impl OlehInfo {
    pub fn parse(atom: &Atom) -> OlehInfo {
        if atom.id() != Id4::PCP_OLEH {
            panic!("this atom is not oleh! {:?}", atom);
        }
        let oleh_atoms = match atom {
            Atom::Child(c) => panic!("this atom is not oleh! {:?}", c),
            Atom::Parent(p) => p,
        };

        let (mut remote_ip, mut agent, mut session_id, mut port, mut version) =
            (None, None, None, None, None);

        for a in oleh_atoms.childs() {
            if let Atom::Parent(p) = a {
                //
            } else if let Atom::Child(c) = a {
                match c.id() {
                    // CHECK
                    Id4::PCP_HELO_REMOTEIP => {
                        //MEMO: IPのデータ構造(BE, LE)どうするか決めてないよねおそらく
                        let ipaddr = match c.len() {
                            4 => {
                                let ip_u32 = c.payload().get_u32_le();
                                IpAddr::V4(Ipv4Addr::from(ip_u32))
                            }
                            16 => {
                                let ip_u128: u128 = c.payload().get_u128();
                                IpAddr::V6(Ipv6Addr::from(ip_u128))
                            }
                            _ => panic!("remote ip length 4(ipv4) or 16(ipv6)"),
                        };
                        remote_ip = Some(ipaddr);
                    }
                    Id4::PCP_HELO_AGENT => {
                        //
                        let s_slice: &[u8] = &c.payload();
                        agent = Some(str::from_utf8(s_slice).unwrap().into());
                    }
                    Id4::PCP_SESSIONID => {
                        let x = c.payload().get_u128();
                        let sid = GnuId::from(x);
                        session_id = Some(sid)
                    }
                    Id4::PCP_HELO_PORT => {
                        let p = c.payload().get_u16_le();
                        port = Some(p);
                    }
                    Id4::PCP_HELO_VERSION => {
                        let ver = c.payload().get_u32_le();
                        version = Some(ver);
                    }
                    _ => {
                        debug!("skip atom({:?})", c)
                    }
                }
            }
        }
        OlehInfo {
            remote_ip, // これは送られてこない場合がある
            agent: agent.unwrap(),
            session_id: session_id.unwrap(),
            port: port.unwrap(),
            version: version.unwrap(),
        }
    }
}

#[cfg(test)]
mod t {
    use super::*;

    #[test]
    fn test_oleh() {
        let sid = GnuId::new();
        let remote_ip: IpAddr = Ipv4Addr::new(127, 0, 0, 1).into();
        let remote_port = 7144;
        let oleh = OlehBuilder::new(sid, remote_ip, remote_port).build();
        let info = OlehInfo::parse(&oleh);
        assert_eq!(info.session_id, sid);
        assert_eq!(info.remote_ip, Some(remote_ip));
        assert_eq!(info.port, remote_port);

        let sid = GnuId::new();
        let remote_ip: IpAddr = Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xc00a, 0x2ff).into();
        let remote_port = 7144;
        let oleh = OlehBuilder::new(sid, remote_ip, remote_port).build();
        let info = OlehInfo::parse(&oleh);
        assert_eq!(info.session_id, sid);
        assert_eq!(info.remote_ip, Some(remote_ip));
        assert_eq!(info.port, remote_port);
    }
}

/*
参考情報

- https://github.com/kumaryu/peercaststation/blob/6184647e600ec3a388462169ab7118314114252e/PeerCastStation/PeerCastStation.PCP/PCPOutputStream.cs#L432
  // remote ip が書き込める場合だけ書き込むっぽい
        if (remoteEndPoint!=null && remoteEndPoint.AddressFamily==channel.NetworkAddressFamily) {
          oleh.SetHeloRemoteIP(remoteEndPoint.Address);
        }
        oleh.SetHeloAgent(peerCast.AgentName);
        oleh.SetHeloSessionID(peerCast.SessionID);
        oleh.SetHeloRemotePort(remote_port);
        PCPVersion.SetHeloVersion(oleh);

- https://github.com/plonk/peercast-yt/blob/787be6405cc2d82a5d26c0023aaa5d1973c13802/core/common/servent.cpp#L1217
        atom.writeString(PCP_HELO_AGENT, PCX_AGENT);
        atom.writeBytes(PCP_HELO_SESSIONID, servMgr->sessionID.id, 16);
        atom.writeInt(PCP_HELO_VERSION, PCP_CLIENT_VERSION);
        atom.writeAddress(PCP_HELO_REMOTEIP, rhost.ip);
        atom.writeShort(PCP_HELO_PORT, rhost.port);
*/
