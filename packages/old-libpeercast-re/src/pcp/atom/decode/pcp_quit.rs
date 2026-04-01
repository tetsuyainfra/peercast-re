use std::sync::Arc;

use crate::{
    error::AtomParseError,
    pcp::{builder::QuitInfo, Atom, Id4},
};

#[derive(Debug, Clone)]
pub struct PcpQuit {
    atom: Arc<Atom>,
    quit: QuitInfo,
}

impl PcpQuit {
    pub fn quit(&self) -> &QuitInfo {
        &self.quit
    }

    #[tracing::instrument]
    pub fn parse(atom: &Atom) -> Result<Self, AtomParseError> {
        if !(atom.id() == Id4::PCP_QUIT && atom.is_child() && atom.as_child().payload().len() == 4)
        {
            return Err(AtomParseError::ValueError);
        }

        let atom = atom.clone();
        let quit = QuitInfo::parse(&atom);

        let pcp_quit = PcpQuit {
            atom: Arc::new(atom),
            quit: quit,
        };

        Ok(pcp_quit)
    }
}
