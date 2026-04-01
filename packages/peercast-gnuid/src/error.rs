#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GnuIdParseError {
    /// 16進数の文字列を数値に変換できない場合
    ParseIntError,

    /// 文字列の桁数が不適切な場合
    NumberOfDigit,
}

impl std::fmt::Display for GnuIdParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GnuIdParseError::ParseIntError => write!(f, "Failed to parse the string as a hexadecimal number."),
            GnuIdParseError::NumberOfDigit => write!(f, "The string must be exactly 32 hexadecimal digits long."),
        }
    }
}
