use std::{
    fmt::{self, Display},
    num::ParseIntError,
    ops::Range,
    str::FromStr,
};

use serde::Serialize;
use thiserror::Error;
use tracing_subscriber::field::debug;

use crate::error;

// use serde::ser::{Serialize, Serializer};

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct GnuId(pub u128);

impl GnuId {
    pub const NONE: GnuId = GnuId(0_u128);

    pub fn new() -> Self {
        // let v = uuid::Uuid::new_v4().as_u128();
        // debug_assert_ne!(v, Self::NONE.0);
        // GnuId(v)
        GnuId(uuid::Uuid::now_v7().as_u128())
    }
    pub fn new_arc() -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self::new())
    }

    pub fn is_none(&self) -> bool {
        self.0 == Self::NONE.0
    }

    #[cfg(debug_assertions)]
    pub fn with(n: u128) -> Self {
        GnuId(n)
    }
}

impl Default for GnuId {
    fn default() -> Self {
        GnuId::new()
    }
}

impl fmt::Debug for GnuId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GnuId({:032X})", self.0)
    }
}

impl fmt::Display for GnuId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        if let Some(precision) = f.precision() {
            let s = format!("{:032X}", self.0);
            write!(f, "{}..", &s[0..precision])
            // for c in s.as_str().chars() {
            //     write!(f, "{}", c)?
            // }
            // write!(f, "..")
        } else {
            write!(f, "{:032X}", self.0)
        }
    }
}

impl FromStr for GnuId {
    type Err = GnuIdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 32 {
            return Err(GnuIdParseError::NumberOfDigit);
        }
        let number = u128::from_str_radix(s, 16)?;
        Ok(GnuId(number))
    }
}

impl From<&GnuId> for String {
    fn from(item: &GnuId) -> Self {
        converter::conv_std(&item.0)
    }
}
impl From<GnuId> for String {
    fn from(item: GnuId) -> Self {
        converter::conv_std(&item.0)
    }
}

impl From<u128> for GnuId {
    fn from(item: u128) -> Self {
        GnuId(item)
    }
}

impl From<GnuId> for u128 {
    fn from(item: GnuId) -> Self {
        item.0
    }
}

impl Serialize for GnuId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // 標準だと[10]みたいな配列+数値形式にシリアライズされる
        // use serde::ser::SerializeTupleStruct;
        // let mut ts = serializer.serialize_tuple_struct("GnuId", 1)?;
        // ts.serialize_field(&self.0)?;
        // ts.end()
        let s = String::from(self);
        serializer.serialize_str(&s)
    }
}
// impl Deserialize for GnuId {}

#[derive(Debug, Error)]
pub enum GnuIdParseError {
    #[error("Parse failed")]
    ParseIntError(#[from] ParseIntError),

    #[error("Not enough or too many digits in string")]
    NumberOfDigit,
}

// Format Precisionで表現できるからいらないか？
/// HumanReadableな表示用に短くしたもの
// #[derive(Debug)]
// pub struct ShortId<'a>(&'a GnuId);
// trait GnuIdExt {
//     fn short_id<'a>(&'a self) -> ShortId<'a>;
// }
// impl GnuIdExt for GnuId {
//     fn short_id<'a>(&'a self) -> ShortId<'a> {
//         ShortId(&self)
//     }
// }
// impl Display for ShortId<'_> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
//         let bytes = self.0 .0.to_be_bytes();
//         write!(f, "{:?}", &bytes[0..4]);
//     }
// }

mod converter {
    #[allow(dead_code)]
    pub fn conv_std(v: &u128) -> String {
        let hex_str: String = v
            .to_be_bytes()
            .iter()
            .map(|i| format!("{i:02X}"))
            .collect::<Vec<String>>()
            .join("");
        hex_str
    }

    #[allow(dead_code)]
    pub fn conv_std_buf(v: &u128) -> String {
        use std::fmt::Write as FmtWrite;

        let mut hex_str = String::with_capacity(32);
        v.to_be_bytes().iter().for_each(|i| {
            write!(&mut hex_str, "{:02X}", i).unwrap();
        });
        hex_str
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gnuid() {
        assert_ne!(GnuId::new(), GnuId::new());

        let id = GnuId::new();
        assert_eq!(id, GnuId::from(id.0.clone()));
        let id_string = id.to_string();
        assert_eq!(id_string, GnuId::from_str(&id_string).unwrap().to_string());

        let id1 = GnuId::new();
        let mut id2 = id1.clone();
        assert_eq!(id1, id2);
        id2.0 = 0x0_u128;
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_from_str() {
        let s = String::from("0000000000000000000000000000000A");
        let g: GnuId = GnuId::from_str("0000000000000000000000000000000A").unwrap();
        assert_eq!(g.0, 10);
        assert_eq!(s, g.to_string());

        // println!("{:?}", &g);

        assert_eq!(
            GnuId::from_str("1A00000000000000000000000000000B")
                .unwrap()
                .0,
            0x1A00000000000000000000000000000B_u128
        );

        // presicion表現
        let id = GnuId::from_str("1A00000000000000000000000000000B").unwrap();
        assert_eq!(format!("{:.6}", id), String::from("1A0000.."));
        assert_eq!(format!("{:.4}", id), String::from("1A00.."));
    }

    #[test]
    fn test_for_serde() {
        let g: GnuId = GnuId::from_str("0000000000000000000000000000000A").unwrap();
        dbg!(&g);
        println!("{}", &g);
        let serialized = serde_json::to_string(&g).unwrap();
        println!("serialized = {}", serialized);
    }

    #[test]
    fn test_short_id() {
        let gid = GnuId::new();
        // let id = gid.short_id();
        println!("{:}", &gid);
        // println!("{}", &id);
        println!("{:.4}", gid)
    }
}
