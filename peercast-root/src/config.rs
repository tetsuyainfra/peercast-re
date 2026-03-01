use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{TomlConfigError, model::IndexInfo};

/// DB設定
#[allow(dead_code)]
#[derive(Debug, Clone)]
struct DbConfig {
    path: PathBuf,
}

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
