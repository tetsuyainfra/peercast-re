use chrono::{DateTime, Utc};

use super::DbIpAddr;


#[derive(Debug)]
pub struct CheckedHost {
    pub id: Option<i64>,
    pub ip: DbIpAddr,
    pub created_at: DateTime<Utc>,
    // pub hostname: DbIpAddr,
    // pub is_active: bool,
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
