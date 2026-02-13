mod index_info;

use clap::ValueEnum;
pub use index_info::{FooterToml, IndexInfo};
use thiserror::Error;

pub mod config;
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
    None,

    /// 疎通OK
    PortCheck,

    /// 疎通OK, 配信速度OK
    BroadcastSpeed,

    /// 疎通OK, 配信速度OK, 規制速度OK(内部の値はアップロード速度[KBps])
    RestrictSpeed,
}

/// ポートチェックの制限速度で設定できる最小値（この値は含めない）
pub static YP_LIMIT_SPEED_MIN: u32 = 499; // 500KBps

/// ポートチェックされたPeerCastの疎通レベル
#[repr(i8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PortLevel {
    /// ポートチェックしたが疎通できなかった
    Incomplete = -1,

    /// ポートチェックが行われていない
    None = 0,

    /// 疎通OK
    Welldone = 1,

    // 疎通OK, 配信速度OK
    WelldoneWithSpeed(u16) = 2,
}

#[derive(Debug, Error)]
pub enum TomlConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML deserialize error: {0}")]
    Toml(#[from] toml::de::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_restrect_level() {
        // assert_eq!(PortRestrictLevel::None, 0_u8);
        // assert_eq!(PortRestrictLevel::Welldone, 1_u8);
        // assert_eq!(PortRestrictLevel::WelldoneReachedUploadSpeed , 2_u8);
        // assert_eq!(PortRestrictLevel::WelldoneReachedRestrictSpeed(1000), 3_u8);
    }

    #[test]
    fn test_port_level() {
        assert!(PortLevel::Incomplete == PortLevel::Incomplete);
        assert!(PortLevel::Incomplete < PortLevel::None);
        assert!(PortLevel::Incomplete < PortLevel::Welldone);
        assert!(PortLevel::Incomplete < PortLevel::WelldoneWithSpeed(0));
        //
        assert!(PortLevel::None < PortLevel::Welldone);
        assert!(PortLevel::None < PortLevel::WelldoneWithSpeed(0));
        //
        assert!(PortLevel::Welldone < PortLevel::WelldoneWithSpeed(0));
        //
        assert!(PortLevel::WelldoneWithSpeed(0) < PortLevel::WelldoneWithSpeed(1));
        assert!(PortLevel::WelldoneWithSpeed(1) > PortLevel::WelldoneWithSpeed(0));
        assert!(PortLevel::WelldoneWithSpeed(1) == PortLevel::WelldoneWithSpeed(1));
        assert!(PortLevel::WelldoneWithSpeed(0) != PortLevel::WelldoneWithSpeed(1));
    }
}
