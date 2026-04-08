use crate::GnuIdParseError;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct GnuId(pub u128);

impl GnuId {
    pub const NONE: GnuId = GnuId(0_u128);

    pub fn new() -> Self {
        // GnuId(uuid::Uuid::now_v7().as_u128()) //
        GnuId(uuid::Uuid::new_v4().as_u128())
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

    pub fn zero() -> Self {
        GnuId(0_u128)
    }
}

impl Default for GnuId {
    fn default() -> Self {
        GnuId::new()
    }
}

impl std::fmt::Debug for GnuId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GnuId({:032X})", self.0)
    }
}

impl std::fmt::Display for GnuId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(precision) = f.precision() {
            let s = format!("{:032X}", self.0);
            write!(f, "{}..", &s[0..precision])
        } else {
            write!(f, "{:032X}", self.0)
        }
    }
}

impl std::str::FromStr for GnuId {
    type Err = GnuIdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 32 {
            return Err(GnuIdParseError::NumberOfDigit);
        }
        let number = u128::from_str_radix(s, 16).map_err(|_| GnuIdParseError::ParseIntError)?;
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

impl From<[u8; 16]> for GnuId {
    fn from(item: [u8; 16]) -> Self {
        GnuId(u128::from_be_bytes(item))
    }
}

impl From<GnuId> for u128 {
    fn from(item: GnuId) -> Self {
        item.0
    }
}

mod converter {

    // Too slow
    // #[allow(dead_code)]
    // pub fn conv_std(v: &u128) -> String {
    //     let hex_str: String = v.to_be_bytes().iter().map(|i| format!("{i:02X}")).collect::<Vec<String>>().join("");
    //     hex_str
    // }

    /// 高速化版
    // fn conv_std(v: &u128) -> String {
    //     let bytes = v.to_be_bytes();
    //     let mut hex_str = String::with_capacity(32);

    //     for b in &bytes {
    //         use std::fmt::Write;
    //         write!(&mut hex_str, "{:02X}", b).unwrap();
    //     }
    //     hex_str
    // }

    /// さらに高速化版
    pub fn conv_std(v: &u128) -> String {
        const HEX: &[u8; 16] = b"0123456789ABCDEF";

        let bytes = v.to_be_bytes();
        let mut buf = [0u8; 32];

        for (i, &b) in bytes.iter().enumerate() {
            buf[i * 2] = HEX[(b >> 4) as usize] as u8;
            buf[i * 2 + 1] = HEX[(b & 0x0F) as usize] as u8;
        }

        unsafe { std::str::from_utf8_unchecked(&buf) }.to_string()
    }
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

#[cfg(test)]
mod tests {
    use std::str::FromStr;

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
            GnuId::from_str("1A00000000000000000000000000000B").unwrap().0,
            0x1A00000000000000000000000000000B_u128
        );

        // presicion表現
        let id = GnuId::from_str("1A00000000000000000000000000000B").unwrap();
        assert_eq!(format!("{:.6}", id), String::from("1A0000.."));
        assert_eq!(format!("{:.4}", id), String::from("1A00.."));
    }

    #[ignore = "it were not implemented yet"]
    #[test]
    fn test_short_id() {
        let gid = GnuId::new();
        // let id = gid.short_id();
        println!("{:}", &gid);
        // println!("{}", &id);
        println!("{:.4}", gid)
    }
}
