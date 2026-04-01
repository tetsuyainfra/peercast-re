#![allow(unused)]

/* usage:
   assert_sync::<TargetStruct>();
*/
pub fn assert_sized<T: Sized>() {}
pub fn assert_sync<T: Sync>() {}
pub fn assert_send<T: Send>() {}
pub fn assert_copy<T: Copy>() {}
pub fn assert_clone<T: Clone>() {}

pub fn init_logger(env_format: &str) {
    use std::sync::OnceLock;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{EnvFilter, fmt};

    // SQL Queryだけログを出したい場合は、次のように環境変数を設定する
    // RUST_LOG=sqlx::query=info

    static INIT_LOGGER: OnceLock<bool> = OnceLock::new();
    let _v = INIT_LOGGER.get_or_init(|| {
        tracing_subscriber::registry()
            .with(fmt::layer().with_file(true).with_line_number(true).with_target(false))
            .with(EnvFilter::from(env_format))
            .init();
        true
    });
}
