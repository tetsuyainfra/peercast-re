use std::{fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, de::Visitor};

use crate::GnuId;

impl Serialize for GnuId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = String::from(self);
        serializer.serialize_str(&s)
    }

    // 高速化版
    // こんな感じで早くできるハズだけどそんなに使わないしいいかな？
    // fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    // where
    //     S: serde::Serializer,
    // {
    //     #[inline]
    //     fn conv(v: &u128) -> [u8; 32] {
    //         const HEX: &[u8; 16] = b"0123456789ABCDEF";

    //         let bytes = v.to_be_bytes();
    //         let mut buf = [0u8; 32];

    //         for (i, &b) in bytes.iter().enumerate() {
    //             buf[i * 2] = HEX[(b >> 4) as usize] as u8;
    //             buf[i * 2 + 1] = HEX[(b & 0x0F) as usize] as u8;
    //         }
    //         buf
    //     }

    //     let chars = conv(&self.0);
    //     let s = unsafe { std::str::from_utf8_unchecked(&chars) };
    //     serializer.serialize_str(&s)
    // }
}

struct IGnuIdVisitor;

impl<'de> Visitor<'de> for IGnuIdVisitor {
    type Value = GnuId;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "an str between 0 and 2^128")
    }

    fn visit_str<E>(self, value: &str) -> Result<GnuId, E>
    where
        E: serde::de::Error,
    {
        let id = GnuId::from_str(value);
        id.map_err(|e| E::custom(format!("{}", e)))
    }
}

impl<'de> Deserialize<'de> for GnuId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(IGnuIdVisitor)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::GnuId;

    #[test]
    fn test_for_serde() {
        let g: GnuId = GnuId::from_str("1234567890000000000000000000000A").unwrap();
        // dbg!(&g);

        let serialized = serde_json::to_string(&g).unwrap();
        assert_eq!(&serialized, &"\"1234567890000000000000000000000A\"");
        // println!("serialized = {}", &serialized);

        let de_g: GnuId = serde_json::from_str(&serialized).unwrap();
        assert_eq!(g, de_g);
    }
}
