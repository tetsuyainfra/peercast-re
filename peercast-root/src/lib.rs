use clap::ValueEnum;
use thiserror::Error;

pub mod channel;
pub mod config;
pub mod connection;
pub mod db;
pub mod filter;
pub mod model;
pub mod prelude;
pub mod repository;
pub mod service;
pub mod test_helper;

//HACKME: std::process:ExitCodeやimpl Terminateを使ったほうがいい？
#[repr(i32)]
pub enum ExitCode {
    Success = 0,
    Failure = 1,
}

/// ポートチェック制限レベル
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum RestrictPortLevel {
    /// ポートチェックを行わない
    None = 0,

    /// 疎通OK
    PortCheck = 1,

    /// 疎通OK, 配信速度OK(配信ビットレート基準)
    BroadcastSpeed = 2,

    /// 疎通OK, 配信速度OK(YP指定規制速度OK)
    RestrictSpeed = 3,
}
impl From<usize> for RestrictPortLevel {
    fn from(value: usize) -> Self {
        match value {
            0 => RestrictPortLevel::None,
            1 => RestrictPortLevel::PortCheck,
            2 => RestrictPortLevel::BroadcastSpeed,
            3 => RestrictPortLevel::RestrictSpeed,
            _ => RestrictPortLevel::RestrictSpeed,
        }
    }
}

/// ポートチェックの制限速度で設定できる最小値（この値は含めない）
pub static YP_LIMIT_SPEED_MIN: u32 = 499; // 500KBps

#[derive(Debug, Error)]
pub enum TomlConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML deserialize error: {0}")]
    Toml(#[from] toml::de::Error),
}
