use clap::ValueEnum;
use thiserror::Error;

pub mod channel;
pub mod config;
pub mod connection;
pub mod db;
pub mod filter;
mod init;
pub mod model;
pub mod prelude;
pub mod repository;
pub mod service;
pub mod test_helper;
pub mod utils;

pub use init::init;

//HACKME: std::process:ExitCodeやimpl Terminateを使ったほうがいい？
#[repr(i32)]
pub enum ExitCode {
    Success = 0,
    Failure = 1,
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
