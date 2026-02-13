use std::path::PathBuf;

/// DB設定
#[allow(dead_code)]
#[derive(Debug, Clone)]
struct DbConfig {
    path: PathBuf,
}
