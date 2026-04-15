////////////////////////////////////////////////////////////////////////////////
/// AtomParseError
/// Atomのパースエラーを表す列挙型
/// 主にcrate::parser::try_parse_atomで使用される
#[derive(Debug, PartialEq, Eq)]
pub enum AtomParseError {
    // 予期しない終端
    UnexpectedEof,

    // /// 不正なフォーマット
    // // InvalidFormat,
    /// 不正なデータ(制限容量以上)
    /// 制限以上の子Atom数、制限以上のペイロードサイズ、過剰な入れ子の深さなど
    MalformedPayload {
        reason: &'static str,
    },
}
impl std::error::Error for AtomParseError {}

impl std::fmt::Display for AtomParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AtomParseError::UnexpectedEof => write!(f, "Unexpected end of input"),
            AtomParseError::MalformedPayload {
                reason,
            } => write!(f, "Malformed payload: {}", reason),
        }
    }
}

/// AtomMut構造操作時のエラー
#[derive(Debug)]
pub enum AtomMutError {
    NotAParent,
}

impl std::error::Error for AtomMutError {}

impl std::fmt::Display for AtomMutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AtomMutError::NotAParent => write!(f, "Not a parent"),
        }
    }
}

/// AtomMutからAtomへの変換時のエラー
#[derive(Debug)]
pub enum AtomFreezeError {
    /// children count {0} exceeds maximum (0x7FFFFFFF)
    TooManyChildren(usize),

    /// payload size {0} exceeds maximum (0x7FFFFFFF)
    PayloadTooLarge(usize),
}

/// AtomCodecのエラー
#[cfg(feature = "codec")]
#[derive(Debug)]
pub enum AtomCodecError {
    Io(std::io::Error),
    Parse(AtomParseError),
    Freeze(AtomFreezeError),
}

impl std::fmt::Display for AtomCodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AtomCodecError::Io(e) => write!(f, "IO error: {}", e),
            AtomCodecError::Parse(e) => write!(f, "Atom parse error: {}", e),
            AtomCodecError::Freeze(e) => write!(f, "Atom freeze error: {:?}", e),
        }
    }
}

impl std::error::Error for AtomCodecError {}

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

impl From<AtomFreezeError> for AtomCodecError {
    fn from(e: AtomFreezeError) -> Self {
        Self::Freeze(e)
    }
}
