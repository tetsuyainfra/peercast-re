use chrono::{DateTime, Utc};

use super::DbIpAddr;

#[derive(Debug)]
pub struct CheckedHost {
    pub id: Option<i64>,
    pub ip_address: DbIpAddr,
    pub port: u16,
    pub port_level: i32,
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
