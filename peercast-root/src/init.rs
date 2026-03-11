use std::sync::OnceLock;

pub(crate) static START_TIME: OnceLock<std::time::Instant> = OnceLock::new();

pub fn init() {
    START_TIME.set(std::time::Instant::now()).ok();
}
