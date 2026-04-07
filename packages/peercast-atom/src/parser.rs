use crate::{AtomKind, error::AtomParseError};

const ATOM_HEADER_LENGTH: usize = 8;
pub const ATOM_DEFAULT_MAX_CHILDS_NUM: u32 = 100;
pub const ATOM_DEFAULT_MAX_PAYLOAD_LENGTH: u32 = 2 * 1024 * 1024; // 2MB
pub const ATOM_MAX_DEPTH: u32 = 128;

#[derive(Debug)]
pub struct AtomParser {
    context: ParseContext,
}

impl Default for AtomParser {
    fn default() -> Self {
        Self {
            context: ParseContext::default(),
        }
    }
}

impl AtomParser {
    pub fn new(max_payload_length: u32, max_childs_num: u32) -> Self {
        Self {
            context: ParseContext {
                max_payload_length,
                max_childs_num,
                ..Default::default()
            },
        }
    }

    pub fn try_parse(&self, buf: &[u8]) -> Result<usize, AtomParseError> {
        _try_parse(&self.context, 0, buf)
    }

    /// 信用されていないバッファから、Atomを検証しつつ取得する
    /// 成功した場合、Atomのバイトサイズを返す
    /// データが途中の場合、Err(AtomParseError::UnexpectedEof)を返す
    /// 内容に不正がある場合、エラーが返ります
    pub fn default_try_parse(buf: &[u8]) -> Result<usize, AtomParseError> {
        _try_parse(&ParseContext::default(), 0, buf)
    }
}

#[derive(Debug)]
struct ParseContext {
    max_payload_length: u32,
    max_childs_num: u32,
    max_depth: u32,
}

impl ParseContext {
    const fn default() -> Self {
        Self {
            max_payload_length: ATOM_DEFAULT_MAX_PAYLOAD_LENGTH,
            max_childs_num: ATOM_DEFAULT_MAX_CHILDS_NUM,
            max_depth: ATOM_MAX_DEPTH,
        }
    }
}

impl Default for ParseContext {
    fn default() -> Self {
        Self::default()
    }
}

fn _try_parse(ctx: &ParseContext, depth: usize, buf: &[u8]) -> Result<usize, AtomParseError> {
    if depth > ctx.max_depth as usize {
        return Err(AtomParseError::MalformedPayload);
    }

    if buf.len() < ATOM_HEADER_LENGTH {
        return Err(AtomParseError::UnexpectedEof);
    }

    let mut arr = [0u8; 4];
    arr.copy_from_slice(&buf[4..8]);
    let size_and_parent = u32::from_le_bytes(arr);

    let length: u32 = size_and_parent & 0x7FFF_FFFF;
    let kind = if (size_and_parent & 0x80000000) == 0 {
        AtomKind::Child
    } else {
        AtomKind::Parent
    };

    match kind {
        AtomKind::Child => {
            if length > ctx.max_payload_length {
                return Err(AtomParseError::MalformedPayload);
            }
            // ChildAtom の場合、lengthはバイトサイズそのもの
            let expected_size = ATOM_HEADER_LENGTH + length as usize;
            if buf.len() < expected_size {
                return Err(AtomParseError::UnexpectedEof);
            }
            Ok(expected_size)
        }
        AtomKind::Parent => {
            if length > ctx.max_childs_num {
                return Err(AtomParseError::MalformedPayload);
            }

            let mut offset = ATOM_HEADER_LENGTH; // 初期値はこのAtomのヘッダサイズ分移動した所
            for _ in 0..length {
                // 子Atomを順に解析してバイトサイズを合計する
                let child_size = _try_parse(ctx, depth + 1, &buf[offset..])?;
                offset += child_size;
            }
            Ok(offset)
        }
    }
}

#[cfg(test)]
mod t {

    use crate::parser::{ATOM_DEFAULT_MAX_CHILDS_NUM, ATOM_DEFAULT_MAX_PAYLOAD_LENGTH, AtomParseError, AtomParser};

    #[test]
    fn test_parser() {
        let parser = AtomParser::default();

        /* タグと長さの途中までしかデータがない場合 */
        let bytes = b"";
        let byte_size = parser.try_parse(bytes);
        assert_eq!(byte_size, Err(AtomParseError::UnexpectedEof));

        let bytes = b"pcp";
        let byte_size = parser.try_parse(bytes);
        assert_eq!(byte_size, Err(AtomParseError::UnexpectedEof));

        let bytes = b"pcp\n\x00\x00\x00";
        let byte_size = parser.try_parse(bytes);
        assert_eq!(byte_size, Err(AtomParseError::UnexpectedEof));

        /* タグと長さはあって、データがない場合@Child */
        let bytes = b"pcp\n\x00\x00\x00\x00";
        let byte_size = parser.try_parse(bytes);
        assert_eq!(byte_size, Ok(8));

        /* タグと長さはあって、データがない場合@Parent */
        let bytes = b"pcp\n\x00\x00\x00\x80";
        let byte_size = parser.try_parse(bytes);
        assert_eq!(byte_size, Ok(8));

        /* タグと長さはあって、データがないが、異常に長い場合/閾値上/Child  */
        let payload_length = ATOM_DEFAULT_MAX_PAYLOAD_LENGTH;
        let mut bytes = Vec::<u8>::new();
        bytes.extend_from_slice(b"pcp\n");
        bytes.extend_from_slice(&payload_length.to_le_bytes());
        let byte_size = parser.try_parse(&bytes);
        assert_eq!(byte_size, Err(AtomParseError::UnexpectedEof));

        /* タグと長さはあって、データがないが、異常に長い場合/閾値越え/Child  */
        let payload_length = ATOM_DEFAULT_MAX_PAYLOAD_LENGTH + 1;
        let mut bytes = Vec::<u8>::new();
        bytes.extend_from_slice(b"pcp\n");
        bytes.extend_from_slice(&payload_length.to_le_bytes());
        let r_byte_size = parser.try_parse(&bytes);
        assert_eq!(r_byte_size, Err(AtomParseError::MalformedPayload));

        /* タグと長さはあって、データがないが、異常に長い場合/閾値上/Parent  */
        let child_length = ATOM_DEFAULT_MAX_CHILDS_NUM | 0x8000_0000;
        let mut bytes = Vec::<u8>::new();
        bytes.extend_from_slice(b"pcp\n");
        bytes.extend_from_slice(&child_length.to_le_bytes());
        let r_byte_size = parser.try_parse(&bytes);
        assert_eq!(r_byte_size, Err(AtomParseError::UnexpectedEof));

        /* タグと長さはあって、データがないが、異常に長い場合/閾値越え/Parent  */
        let child_length = (ATOM_DEFAULT_MAX_CHILDS_NUM + 1) | 0x8000_0000;
        let mut bytes = Vec::<u8>::new();
        bytes.extend_from_slice(b"pcp\n");
        bytes.extend_from_slice(&child_length.to_le_bytes());
        let byte_size = parser.try_parse(&bytes);
        assert_eq!(byte_size, Err(AtomParseError::MalformedPayload));
    }

    #[test]
    fn test_parser_child() {
        let parser = AtomParser::default();

        // ChildAtomが一つ
        let mut bytes = Vec::<u8>::new();
        bytes.extend_from_slice(b"pcp\n");
        bytes.extend_from_slice(4_u32.to_le_bytes().as_slice());
        bytes.extend_from_slice(0xDEADBEEF_u32.to_le_bytes().as_slice());
        let byte_size = parser.try_parse(&bytes);
        assert_eq!(byte_size, Ok(12));
    }

    #[test]
    fn test_parser_parent() {
        let parser = AtomParser::default();

        // Parent
        let mut bytes = Vec::<u8>::new();
        bytes.extend_from_slice(b"pcp\n");
        bytes.extend_from_slice((2_u32 | 0x8000_0000).to_le_bytes().as_slice());
        {
            // Child0
            bytes.extend_from_slice(b"pcp\n");
            bytes.extend_from_slice(2_u32.to_le_bytes().as_slice());
            bytes.extend_from_slice(0xDEAD_u16.to_le_bytes().as_slice());
            // Child1
            bytes.extend_from_slice(b"pcp\n");
            bytes.extend_from_slice(4_u32.to_le_bytes().as_slice());
            bytes.extend_from_slice(0xDEADBEEF_u32.to_le_bytes().as_slice());
        }
        let byte_size = parser.try_parse(&bytes);
        assert_eq!(byte_size, Ok(8 + (8 + 2) + (8 + 4)));
    }
}
