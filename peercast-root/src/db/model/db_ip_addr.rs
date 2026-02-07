use std::net::IpAddr;

use sqlx::{Sqlite, sqlite::SqliteTypeInfo};


#[derive(Debug)]
pub struct DbIpAddr(pub IpAddr);

// 便利にするためのDeref実装（DbIpAddrをIpAddrのように扱える）
impl std::ops::Deref for DbIpAddr {
    type Target = IpAddr;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// 型情報の定義 (BLOB / BYTEA)
impl sqlx::Type<Sqlite> for DbIpAddr
where
    Vec<u8>: sqlx::Type<Sqlite>,
{
    fn type_info() -> SqliteTypeInfo {
        <Vec<u8> as sqlx::Type<Sqlite>>::type_info()
    }
}

////////////////////////////////////////////////////////////////////////////////
// SQLx Encode / Decode
impl<'r> sqlx::Decode<'r, Sqlite> for DbIpAddr {
    fn decode(value: <Sqlite as sqlx::Database>::ValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let bytes = <Vec<u8> as sqlx::Decode<'r, Sqlite>>::decode(value)?;
        let ip = match bytes.len() {
            4 => {
                let mut array = [0u8; 4];
                array.copy_from_slice(&bytes);
                IpAddr::from(array)
            }
            16 => {
                let mut array = [0u8; 16];
                array.copy_from_slice(&bytes);
                IpAddr::from(array)
            }
            _ => return Err("Invalid IP address length".into()),
        };

        Ok(DbIpAddr(ip))
    }
}

impl<'r> sqlx::Encode<'r, Sqlite> for DbIpAddr {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as sqlx::Database>::ArgumentBuffer<'r>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        let bytes: Vec<u8> = match self.0 {
            IpAddr::V4(v4) => v4.octets().to_vec(),
            IpAddr::V6(v6) => v6.octets().to_vec(),
        };

        <Vec<u8> as sqlx::Encode<Sqlite>>::encode(bytes, buf)
    }
}
