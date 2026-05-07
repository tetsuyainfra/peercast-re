use std::path::PathBuf;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::{TomlConfigError, model::IndexInfo, service::yellow_page::create_user_status_default_function};
// use std::path::PathBuf;

// use serde::{Deserialize, Serialize};

// use crate::{TomlConfigError, model::IndexInfo};

// /// DB設定
// #[allow(dead_code)]
// #[derive(Debug, Clone)]
// struct DbConfig {
//     path: PathBuf,
// }

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

/// チャンネルリストに追加するユーザー情報の形式
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum YpAppendUserStatus {
    /// 追加しない
    None,
    /// 標準形式
    Default,
}

/// チャンネルリストに追加するシステム情報の形式
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum YpAppendSystemStatus {
    /// 追加しない
    None,
    /// 標準形式
    Default,

    /// 標準形式 + アクセスしてきたホストの情報
    WithHost,
}

/// チャンネルリストに追加する管理者がカスタム可能な情報の形式
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct FooterToml {
    #[serde(default)]
    pub infomations: Vec<IndexInfo>,
}

impl FooterToml {
    pub fn from_path(path: &PathBuf) -> Result<Self, TomlConfigError> {
        let s = std::fs::read_to_string(path)?;
        let t = toml::from_str(&s)?;
        Ok(t)
    }
}
