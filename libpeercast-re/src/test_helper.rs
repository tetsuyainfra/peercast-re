#[macro_export]
macro_rules! show_size {
    ($Type:ty) => {
        let size = std::mem::size_of::<$Type>();
        let type_s = stringify!($Type);
        println!("{type_s:20} : {size:3}");
    };
}

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

#[cfg(test)]
mod t {
    use crate::test_helper::assert_clone;

    #[test]
    fn test_helpers() {
        #[derive(Clone)]
        struct C {}
        assert_clone::<C>();
    }
}
