pub mod checked_hosts;

pub use checked_hosts::CheckedHostRepository;

/// ```compile_fail
/// let x: i32 = "Hello";
/// ```
pub fn compile_error_test() {}
