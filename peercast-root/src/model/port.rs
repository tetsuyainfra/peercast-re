use core::fmt;

use sqlx::{Sqlite, sqlite::SqliteTypeInfo};

////////////////////////////////////////////////////////////////////////////////
// PortLevel
//
/// ポートチェックされたPeerCastの疎通レベル
#[repr(i8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PortLevel {
    /// ポートチェックしたが疎通できなかった
    Incomplete = -1,

    /// 疎通OK
    Welldone = 1,
}

impl fmt::Display for PortLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PortLevel::Incomplete => write!(f, "Incomplete"),
            PortLevel::Welldone => write!(f, "Welldone"),
        }
    }
}

impl TryFrom<i8> for PortLevel {
    type Error = anyhow::Error;

    fn try_from(value: i8) -> Result<Self, Self::Error> {
        match value {
            -1 => Ok(PortLevel::Incomplete),
            1 => Ok(PortLevel::Welldone),
            _ => Err(anyhow::anyhow!("Invalid PortLevel value: {}", value)),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// SQLx Encode / Decode
impl sqlx::Type<Sqlite> for PortLevel
where
    Vec<u8>: sqlx::Type<Sqlite>,
{
    fn type_info() -> SqliteTypeInfo {
        <i8 as sqlx::Type<Sqlite>>::type_info()
    }
}

impl<'r> sqlx::Decode<'r, Sqlite> for PortLevel {
    fn decode(value: <Sqlite as sqlx::Database>::ValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let int_value = <i8 as sqlx::Decode<'r, Sqlite>>::decode(value)?;

        match int_value {
            -1 => Ok(PortLevel::Incomplete),
            1 => Ok(PortLevel::Welldone),
            // DBに不正な値が入っていた場合、エラーになっちゃうけどいい？
            _ => Err("Invalid PortLevel value".into()),
        }
    }
}

impl<'r> sqlx::Encode<'r, Sqlite> for PortLevel {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as sqlx::Database>::ArgumentBuffer<'r>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        let int_value: i8 = match self {
            PortLevel::Incomplete => -1,
            PortLevel::Welldone => 1,
        };

        <i8 as sqlx::Encode<Sqlite>>::encode(int_value, buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_restrect_level() {
        // assert_eq!(PortRestrictLevel::None, 0_u8);
        // assert_eq!(PortRestrictLevel::Welldone, 1_u8);
        // assert_eq!(PortRestrictLevel::WelldoneReachedUploadSpeed , 2_u8);
        // assert_eq!(PortRestrictLevel::WelldoneReachedRestrictSpeed(1000), 3_u8);
    }

    #[test]
    fn test_port_level() {
        // assert_eq!(size_of::<PortLevel>(), size_of::<i16>());

        assert!(PortLevel::Incomplete == PortLevel::Incomplete);
        assert!(PortLevel::Incomplete < PortLevel::Welldone);
        // assert!(PortLevel::WelldoneWithSpeed(0) < PortLevel::WelldoneWithSpeed(1));
        // assert!(PortLevel::WelldoneWithSpeed(1) > PortLevel::WelldoneWithSpeed(0));
        // assert!(PortLevel::WelldoneWithSpeed(1) == PortLevel::WelldoneWithSpeed(1));
        // assert!(PortLevel::WelldoneWithSpeed(0) != PortLevel::WelldoneWithSpeed(1));
    }
}
