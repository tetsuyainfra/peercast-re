use chrono::{DateTime, Utc};

use crate::model::PortLevel;

use super::DbIpAddr;

#[derive(Debug)]
pub struct CheckedHost {
    pub id: Option<i64>,
    pub ip_address: DbIpAddr,
    pub port: u16,
    pub port_level: PortLevel,
    pub upload_speed: Option<u16>,
    pub updated_at: DateTime<Utc>,
}

impl CheckedHost {
    // pub fn new(hostname: DbIpAddr, is_active: bool) -> Self {
    //     Self {
    //         id: None,
    //         hostname,
    //         is_active,
    //     }
    // }
}
