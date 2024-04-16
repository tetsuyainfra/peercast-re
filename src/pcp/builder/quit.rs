use bytes::Buf;
use tracing::{error, warn};

use crate::pcp::{builder::quit, error_code::QuitCode, Atom, Id4};

pub enum QuitReason {
    Any,
    SendTimeoutError,
    BadAgentError,
    ConnectionError,
    NotIdentifiedError,
    UnavailableError,
    NoHostOrOffAir,
    // NoHost, OffAir,を統合
    UserShutdown,
}

pub struct QuitBuilder {
    quit_reason: QuitReason,
}

impl QuitBuilder {
    pub fn new(quit_reason: QuitReason) -> Self {
        Self { quit_reason }
    }
    pub fn build(self) -> Atom {
        let reason_u32: u32 = match self.quit_reason {
            QuitReason::Any => QuitCode::ANY,
            QuitReason::SendTimeoutError => QuitCode::SEND_TIMEOUT_ERROR,
            QuitReason::BadAgentError => QuitCode::BAD_AGENT_ERROR,
            QuitReason::ConnectionError => QuitCode::CONNECTION_ERROR,
            QuitReason::NotIdentifiedError => QuitCode::NOT_IDENTIFIED_ERROR,
            QuitReason::UnavailableError => QuitCode::UNAVAILABLE_ERROR,
            QuitReason::NoHostOrOffAir => QuitCode::NO_HOST_OR_OFFAIR,
            QuitReason::UserShutdown => QuitCode::USER_SHUTDOWN,
        };

        Atom::Child((Id4::PCP_QUIT, reason_u32).into())
    }
}

#[derive(Debug)]
pub struct QuitInfo {
    quit_code: u32,
}

impl QuitInfo {
    pub fn reason(&self) -> QuitReason {
        match self.quit_code {
            QuitCode::ANY => QuitReason::Any,
            QuitCode::SEND_TIMEOUT_ERROR => QuitReason::SendTimeoutError,
            QuitCode::BAD_AGENT_ERROR => QuitReason::BadAgentError,
            QuitCode::CONNECTION_ERROR => QuitReason::ConnectionError,
            QuitCode::NOT_IDENTIFIED_ERROR => QuitReason::NotIdentifiedError,
            QuitCode::UNAVAILABLE_ERROR => QuitReason::UnavailableError,
            QuitCode::NO_HOST_OR_OFFAIR => QuitReason::NoHostOrOffAir,
            QuitCode::USER_SHUTDOWN => QuitReason::UserShutdown,
            _ => {
                error!("Can't parse quit code. but NOT CATASTROPIC, return QuiteReason::Any");
                QuitReason::Any
            }
        }
    }

    pub fn parse(atom: &Atom) -> QuitInfo {
        // FIXME: panicはさすがにやりすぎでは？
        if atom.id() != Id4::PCP_QUIT {
            panic!("this atom is not quit! {:?}", atom);
        }
        let quit_atoms = match atom {
            Atom::Child(c) => c,
            Atom::Parent(p) => panic!("this atom is not quit! {:?}", p),
        };
        if quit_atoms.payload().len() != 4 {
            panic!("this atom is not quit! {:?}", atom);
        }

        let quit_code = quit_atoms.payload().get_u32_le();
        QuitInfo { quit_code }
    }
}

/*
参考文献
- https://github.com/kumaryu/peercaststation/blob/6184647e600ec3a388462169ab7118314114252e/PeerCastStation/PeerCastStation.PCP/PCPOutputStream.cs#L440

-
*/
