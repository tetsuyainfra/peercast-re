#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IPVersion {
    V4,
    V6,
}

use std::net::IpAddr;

impl From<IpAddr> for IPVersion {
    fn from(ip: IpAddr) -> Self {
        match ip {
            IpAddr::V4(_) => IPVersion::V4,
            IpAddr::V6(_) => IPVersion::V6,
        }
    }
}
