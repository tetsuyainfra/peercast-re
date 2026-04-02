////////////////////////////////////////////////////////////////////////////////
/// AtomParseError
/// Atomのパースエラーを表す列挙型
/// 主にcrate::parser::try_parse_atomで使用される
#[derive(Debug, PartialEq, Eq)]
pub enum AtomParseError {
    // 予期しない終端
    UnexpectedEof,

    // /// 不正なフォーマット
    // InvalidFormat,
    /// 不正なデータ(制限容量以上)
    MalformedPayload,
    // MalformedPayload(&'static str)
}

////////////////////////////////////////////////////////////////////////////////
/// AtomCodecError
/// AtomCodecのエラーを表す列挙型
#[cfg(feature = "codec")]
#[derive(Debug)]
pub enum AtomCodecError {
    Io(std::io::Error),
    Parse(AtomParseError),
}

impl From<std::io::Error> for AtomCodecError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<AtomParseError> for AtomCodecError {
    fn from(e: AtomParseError) -> Self {
        Self::Parse(e)
    }
}
