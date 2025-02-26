#![allow(unused)]

pub fn is_sized<T: Sized>() {}
pub fn is_sync<T: Sync>() {}
pub fn is_send<T: Send>() {}
pub fn is_copy<T: Copy>() {}
pub fn is_clone<T: Clone>() {}

pub fn init_logger(env_format: &str) {
    use std::sync::OnceLock;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{fmt, EnvFilter};

    static INIT_LOGGER: OnceLock<bool> = OnceLock::new();
    let _v = INIT_LOGGER.get_or_init(|| {
        tracing_subscriber::registry()
            .with(
                fmt::layer()
                    .with_file(true)
                    .with_line_number(true)
                    .with_target(false),
            )
            .with(EnvFilter::from(env_format))
            .init();
        true
    });
}
