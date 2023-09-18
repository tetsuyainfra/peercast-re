//

/// Atomパケットのヘッド部の4バイトから、そのAtomパケットがParentかChildか判定する
#[allow(dead_code)]
#[inline]
pub(super) fn is_parent_packet_size(b: [u8; 4]) -> bool {
    split_packet_size(b).0
}

/// Atomパケットのヘッド部の4バイトから長さを取得する
#[allow(dead_code)]
#[inline]
pub(super) fn get_length_packet_size(b: [u8; 4]) -> u32 {
    split_packet_size(b).1
}

/// Atomパケットのヘッド部の4バイトはLittleEndianになっている。
/// これを処理しているマシンのエンディアンに変換し、符号化を解除する
/// .0 が true なら親
/// .1 は 長さ(.0がParentなら含まれるAtomパケットの数、Childならバイト数)
#[inline]
pub(super) fn split_packet_size_u32(size: u32) -> (bool, u32) {
    let is_parent = (size & 0x8000_0000_u32) > 0;
    let length = size & 0x7FFF_FFFF_u32;

    (is_parent, length)
}

/// split_from_packet_sizeを参照の事
#[inline]
pub(super) fn split_packet_size(b: [u8; 4]) -> (bool, u32) {
    let size: u32 = u32::from_le_bytes(b);
    split_packet_size_u32(size)
}

/// MSBを1にする
#[inline]
pub(super) fn enable_msb_1(length: u32) -> u32 {
    length | 0x8000_0000_u32
}

pub mod atom {
    use bytes::{Buf, Bytes};

    pub fn to_string(b: &Bytes) -> String {
        let mut s = String::from_utf8_lossy(b).to_string();
        // 末尾が\0以外の時だけ元に戻せばよい
        match s.pop() {
            None => {}
            Some('\0') => {}
            Some(other) => s.push(other),
        }
        s
    }

    pub fn to_u32(b: &Bytes) -> u32 {
        b.clone().get_u32_le()
    }
    pub fn to_u32_be(b: &Bytes) -> u32 {
        b.clone().get_u32()
    }

    pub fn to_u8(b: &Bytes) -> u8 {
        b.clone().get_u8()
    }
}

#[cfg(test)]
mod t {
    use super::*;

    #[test]
    fn test_functions() {
        // assert_eq!(split_from_head(0x0000_0001_u32).0, false);
        assert_eq!(split_packet_size([0x00, 0, 0, 0x01]).0, false);

        // assert_eq!(split_from_head(0x8000_0001_u32).0, true);
        assert_eq!(split_packet_size([0x01, 0, 0, 0x80]).0, true);

        // assert_eq!(split_from_head(0xA000_0001_u32).0, true);
        assert_eq!(split_packet_size([0x01, 0, 0, 0xA0]).0, true);

        // assert_eq!(split_from_head(0xA000_0001_u32).1, 0x2000_0001_u32);
        assert_eq!(split_packet_size([0x01, 0, 0, 0xA0]).1, 0x2000_0001_u32);

        assert_eq!(split_packet_size_u32(0x0000_0000), (false, 0x0000_0000_u32));
        assert_eq!(split_packet_size_u32(0x1101_000A), (false, 0x1101_000A_u32));
        assert_eq!(split_packet_size_u32(0x9101_000A), (true, 0x1101_000A_u32));
    }

    #[test]
    fn test_bit() {
        let value = 0x0000_0001_u32;
        let parent = (value & 0x8000_0000_u32) > 0;
        assert_eq!(parent, false);

        let value = 0x8000_0001_u32;
        let parent = (value & 0x8000_0000_u32) > 0;
        assert_eq!(parent, true);

        let value = 0xA000_0001_u32;
        let length = value & 0x7FFF_FFFF_u32;
        assert_eq!(length, 0x2000_0001_u32);

        let value = 0x2000_0001_u32;
        let length = value & 0x7FFF_FFFF_u32;
        assert_eq!(length, 0x2000_0001_u32);
    }
}
