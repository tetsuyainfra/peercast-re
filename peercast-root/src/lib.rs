
mod index_info;

pub use index_info::{IndexInfo, FooterToml};



//HACKME: std::process:ExitCodeやimpl Terminateを使ったほうがいい？
#[repr(i32)]
pub enum ExitCode{
    Success = 0,
    Failure = 1,
}