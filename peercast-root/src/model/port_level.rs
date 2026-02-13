use core::fmt;

use sqlx::{Sqlite, sqlite::SqliteTypeInfo};


/// ポートチェックされたPeerCastの疎通レベル
#[repr(i16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PortLevel {
    /// ポートチェックしたが疎通できなかった
    Incomplete = -1,

    /// ポートチェックが行われていない
    None = 0,

    /// 疎通OK
    Welldone = 1,

    // 疎通OK, 配信速度OK
    WelldoneWithSpeed(i16)
}

impl fmt::Display for PortLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PortLevel::Incomplete => write!(f, "Incomplete"),
            PortLevel::None => write!(f, "None"),
            PortLevel::Welldone => write!(f, "Welldone"),
            PortLevel::WelldoneWithSpeed(speed) => write!(f, "WelldoneWithSpeed({})", speed),
        }
    }
}

impl From<i16> for PortLevel {
    fn from(value: i16) -> Self {
        match value {
            -1 => PortLevel::Incomplete,
            0 => PortLevel::None,
            1 => PortLevel::Welldone,
            v if v >= 2 => PortLevel::WelldoneWithSpeed(v),
            _ => PortLevel::Incomplete, // デフォルト値
        }
    }
}


impl sqlx::Type<Sqlite> for PortLevel
where
    Vec<u8>: sqlx::Type<Sqlite>,
{
    fn type_info() -> SqliteTypeInfo {
        <i16 as sqlx::Type<Sqlite>>::type_info()
    }
}

////////////////////////////////////////////////////////////////////////////////
// SQLx Encode / Decode
impl<'r> sqlx::Decode<'r, Sqlite> for PortLevel {
    fn decode(value: <Sqlite as sqlx::Database>::ValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let int_value = <i16 as sqlx::Decode<'r, Sqlite>>::decode(value)?;

        match int_value {
            -1 => Ok(PortLevel::Incomplete),
            0 => Ok(PortLevel::None),
            1 => Ok(PortLevel::Welldone),
            v if v >= 2 => Ok(PortLevel::WelldoneWithSpeed(v)),
            _ => Err("Invalid PortLevel value".into()),
        }
    }
}

impl<'r> sqlx::Encode<'r, Sqlite> for PortLevel {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as sqlx::Database>::ArgumentBuffer<'r>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        let int_value: i16 = match self {
            PortLevel::Incomplete => -1,
            PortLevel::None => 0,
            PortLevel::Welldone => 1,
            PortLevel::WelldoneWithSpeed(speed) => *speed,
        };

        <i16 as sqlx::Encode<Sqlite>>::encode(int_value, buf)
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
        assert!(PortLevel::Incomplete < PortLevel::None);
        assert!(PortLevel::Incomplete < PortLevel::Welldone);
        assert!(PortLevel::Incomplete < PortLevel::WelldoneWithSpeed(0));
        //
        assert!(PortLevel::None < PortLevel::Welldone);
        assert!(PortLevel::None < PortLevel::WelldoneWithSpeed(0));
        //
        assert!(PortLevel::Welldone < PortLevel::WelldoneWithSpeed(0));
        //
        assert!(PortLevel::WelldoneWithSpeed(0) < PortLevel::WelldoneWithSpeed(1));
        assert!(PortLevel::WelldoneWithSpeed(1) > PortLevel::WelldoneWithSpeed(0));
        assert!(PortLevel::WelldoneWithSpeed(1) == PortLevel::WelldoneWithSpeed(1));
        assert!(PortLevel::WelldoneWithSpeed(0) != PortLevel::WelldoneWithSpeed(1));
    }
}
