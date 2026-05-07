mod chan;
pub mod error;
mod helo;
mod magic;
mod oleh;
mod ping_pong;

use std::cell::LazyCell;

pub use chan::*;
pub use helo::*;
pub use magic::*;
pub use oleh::*;
use peercast_atom::{Atom, AtomMut};
use peercast_id4::Id4;
pub use ping_pong::*;

pub const AGENT: LazyCell<Atom> = LazyCell::new(|| {
    let agent_str: String = crate::PKG_AGENT.into();
    AtomMut::from((Id4::PCP_HELO_AGENT, agent_str)).freeze().unwrap()
});
pub const VERSION: LazyCell<Atom> = LazyCell::new(|| {
    //
    AtomMut::from((Id4::PCP_HELO_AGENT, 1218_u32)).freeze().unwrap()
});
