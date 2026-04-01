/// Dumps a byte slice as a UTF-8 string if possible, otherwise as a debug-formatted byte array.
pub fn dump_str(bytes: &[u8]) -> String {
    match str::from_utf8(bytes) {
        Ok(s) => return s.to_string(),
        Err(_) => return format!("{bytes:?}"),
    }
}
