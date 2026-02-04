use bytes::{Buf, Bytes, BytesMut};

use crate::pcp::atom2::{Atom2, Kind};

pub trait PacketParser {
    type Packet;

    /// 新しいデータを供給
    fn push(&mut self, data: Bytes);

    /// 解析できたパケットを取り出す
    fn next_packet(&mut self) -> Option<Self::Packet>;
}

#[derive(Debug)]
pub enum ParseError {
    /// 不正なフォーマット
    InvalidFormat,
    /// データの終端に達した
    UnexpectedEnd,
    /// 不正なデータ
    MalformedData,
}

const ATOM_HEADER_LENGTH: usize = 8;
const ATOM_MAX_BYTE_LENGTH: u32 = 2 * 1024 * 1024; // 2MB

/// 信用されていないバッファから、Atomを検証しつつ取得する
/// 成功した場合、Atomのバイトサイズを返す
pub(super) fn try_parse_atom(buf: &[u8]) -> Result<usize, ParseError> {
    if buf.len() < ATOM_HEADER_LENGTH {
        return Err(ParseError::UnexpectedEnd);
    }

    let size_and_parent = (&buf[4..8]).get_u32_le();
    let length: u32 = size_and_parent & 0x7FFF_FFFF;
    let kind = if (size_and_parent & 0x80000000) == 0 {
        Kind::Child
    } else {
        Kind::Parent
    };

    if length > ATOM_MAX_BYTE_LENGTH {
        // 2MBを超えるAtomは不正とみなす
        return Err(ParseError::MalformedData);
    }

    match kind {
        Kind::Child => {
            // ChildAtom の場合、lengthはバイトサイズそのもの
            let expected_size = ATOM_HEADER_LENGTH + length as usize;
            if buf.len() < expected_size {
                return Err(ParseError::UnexpectedEnd);
            }
            Ok(expected_size)
        }
        Kind::Parent => {
            let mut offset = ATOM_HEADER_LENGTH; // 初期値はこのAtomのヘッダサイズ
            for _ in 0..length {
                if offset >= buf.len() {
                    return Err(ParseError::UnexpectedEnd);
                }
                // 子Atomを順に解析してバイトサイズを合計する
                let child_size = try_parse_atom(&buf[offset..])?;
                offset += child_size;
            }
            Ok(offset)
        }
    }
}
