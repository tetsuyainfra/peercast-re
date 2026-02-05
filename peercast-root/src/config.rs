use std::path::PathBuf;

/// DB設定
#[derive(Debug, Clone)]
struct DbConfig {
    path: PathBuf,
}
