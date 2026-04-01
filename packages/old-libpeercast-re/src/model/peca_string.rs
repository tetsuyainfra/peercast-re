use core::fmt;
use std::sync::OnceLock;

use chardetng::EncodingDetector;
use encoding_rs::Encoding;

struct PecaString {
    raw: Vec<u8>,
    guess: OnceLock<GuessText>,
}
struct GuessText {
    text: String,
    encoding: &'static Encoding,
}

impl PecaString {
    fn new(raw: Vec<u8> /*encode: Option<>*/) -> Self {
        Self {
            raw,
            guess: OnceLock::new(),
        }
    }

    fn decode(&self) -> &str {
        let guess = self.guess.get_or_init(|| {
            // エンコーディングを検出
            let mut detector = EncodingDetector::new();
            let _ = detector.feed(&self.raw, true);
            let decoder = detector.guess(None, true);

            // UTF-8にデコードする
            let (text, encoding, _success) = decoder.decode(&self.raw);
            let guess = GuessText {
                text: text.to_string(),
                encoding,
            };
            guess
        });

        &guess.text
    }

    pub fn decode_cow(&self) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::from(self.decode())
    }
}

impl std::fmt::Display for PecaString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.decode()))
        // f.write_str(&self.decode())
    }
}

impl std::fmt::Debug for PecaString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("PecaString").field(&self.decode()).finish()
    }
}

#[cfg(test)]
mod t {
    use crate::model::peca_string::PecaString;

    fn to_mixed_hex(bytes: &[u8]) -> String {
        let mut out = String::new();

        for &b in bytes {
            if b.is_ascii_alphanumeric() {
                out.push(b as char);
            } else {
                out.push_str(&format!("\\x{:02X}", b));
            }
        }

        out
    }

    #[test]
    fn test_print() {
        let s = PecaString::new(vec![
            80, 101, 101, 114, 67, 97, 115, 116, 83, 116, 97, 116, 105, 111, 110, 47, 54, 46, 48, 46, 49, 46, 48, 0,
        ]);

        println!("{}", s);
        println!("{}", to_mixed_hex(s.decode().as_bytes()));
    }
}
