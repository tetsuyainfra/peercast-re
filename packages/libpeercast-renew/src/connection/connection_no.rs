use std::{fmt, sync::atomic::AtomicU32};

static GLOBAL_CONNECTION_COUNT: AtomicU32 = AtomicU32::new(1);

/// ConnectionNoは接続ごとに一意なIDを表す型です。
/// 接続が確立されるたびに、ConnectionNo::new()を呼び出して新しいIDを生成します。
/// ConnectionNoはi32型の整数を内部に持ちますが、生成されるIDは1から始まり、i32::MAXまで増加します。
/// i32::MINは負の値であるため、IDの生成がi32::MAXを超えると、IDはi32::MINから再び増加し始めます。
/// これにより、IDの生成は循環することになりますが、i32::MAXを超えると警告が表示されるため、アプリケーションの再起動が推奨されます。
/// このi32は依存クレートがi32を使用しているため、i32を使用しています。
/// TODO: そのうち解消出来たらいいな！
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConnectionNo(pub i32);

impl ConnectionNo {
    pub fn new() -> Self {
        let count = GLOBAL_CONNECTION_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if count > i32::MAX as u32 {
            tracing::warn!("connection id counter overflowed i32::MAX. you should reboot this application.");
        }
        Self(count as i32)
    }
}

impl Default for ConnectionNo {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ConnectionNo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Connection({})", self.0))
    }
}

// もし必要なら、i32からConnectionNoへの変換を実装できますが、現時点では必要ないため、コメントアウトしています。
// 下のfrom_raw関数の方が明示的でわかりやすいかもしれません。
// impl From<i32> for ConnectionNo {
//     fn from(value: i32) -> Self {
//         Self(value)
//     }
// }
// From<i32> の代わりに明示的な名前にする案
// pub fn from_raw(value: i32) -> Self {
//     Self(value)
// }

#[cfg(test)]
mod t {
    use crate::show_size;

    use super::*;

    #[test]
    fn test_connection_id() {
        let id = ConnectionNo::new();
        assert_eq!(id.0, 1);
        let id = ConnectionNo::new();
        assert_eq!(id.0, 2);

        let x_max = AtomicU32::new(i32::MAX as u32);
        let count = x_max.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(count, i32::MAX as u32);

        let count = x_max.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(count, i32::MIN as u32);
    }

    #[ignore = "this is show size. not test"]
    #[test]
    fn test_size() {
        show_size!(i32);
        show_size!(ConnectionNo);
    }
}
