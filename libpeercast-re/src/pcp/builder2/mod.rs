mod broadcast;
mod chan;
mod error;
mod helo;
mod host;
mod ok;
mod oleh;
mod ping_pong;
mod root;

pub use broadcast::*;
pub use chan::*;
pub use error::*;
pub use helo::*;
pub use host::*;
pub use ok::*;
pub use oleh::*;
pub use ping_pong::*;
pub use root::*;

use crate::pcp::{Atom2, AtomMut, Id4};
use std::cell::LazyCell;

pub const AGENT: LazyCell<Atom2> = LazyCell::new(|| {
    let agent_str: Vec<u8> = crate::PKG_AGENT.into();
    AtomMut::from((Id4::PCP_HELO_AGENT, agent_str)).into()
});
pub const VERSION: LazyCell<Atom2> = LazyCell::new(|| {
    //
    AtomMut::from((Id4::PCP_HELO_AGENT, 1218_u32)).into()
});
