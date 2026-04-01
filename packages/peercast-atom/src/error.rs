#[derive(Debug, PartialEq, Eq)]
pub enum AtomParseError {
    // 予期しない終端
    UnexpectedEof,

    // /// 不正なフォーマット
    // InvalidFormat,
    /// 不正なデータ(制限容量以上)
    MalformedPayload,
}
