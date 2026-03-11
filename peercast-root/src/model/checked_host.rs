use std::net::IpAddr;

use chrono::{DateTime, Utc};

use crate::model::PortLevel;

use super::DbIpAddr;

#[derive(Debug, PartialEq, Eq)]
pub struct CheckedHost {
    pub id: Option<i64>,
    pub ip_address: DbIpAddr,
    pub port: u16,
    pub port_level: PortLevel,
    pub upload_speed: Option<u32>,
    pub updated_at: DateTime<Utc>,
}

impl CheckedHost {
    pub fn new(
        ip_address: IpAddr,
        port: u16,
        port_level: PortLevel,
        upload_speed: Option<u32>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: None,
            ip_address: DbIpAddr(ip_address),
            port,
            port_level,
            upload_speed,
            updated_at,
        }
    }
}
