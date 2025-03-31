#[allow(non_snake_case)]
pub(self) mod RawQuitCode {
    pub const PCP_ERROR_QUIT: u32 = 1000;

    pub const PCP_ERROR_SKIP: u32 = 1;
    pub const PCP_ERROR_BADAGENT: u32 = 7;
    pub const PCP_ERROR_READ: u32 = 3000;
    pub const PCP_ERROR_NOTIDENTIFIE: u32 = 5;
    pub const PCP_ERROR_UNAVAILABLE: u32 = 3;
    pub const PCP_ERROR_OFFAIR: u32 = 8;
    pub const PCP_ERROR_SHUTDOWN: u32 = 9;
}

use RawQuitCode::*;
pub struct QuitCode;
impl QuitCode {
    pub const ANY: u32 = PCP_ERROR_QUIT;
    pub const SEND_TIMEOUT_ERROR: u32 = PCP_ERROR_QUIT + PCP_ERROR_SKIP;
    pub const BAD_AGENT_ERROR: u32 = PCP_ERROR_QUIT + PCP_ERROR_BADAGENT;
    pub const CONNECTION_ERROR: u32 = PCP_ERROR_QUIT + PCP_ERROR_READ;
    pub const NOT_IDENTIFIED_ERROR: u32 = PCP_ERROR_QUIT + PCP_ERROR_NOTIDENTIFIE;
    pub const UNAVAILABLE_ERROR: u32 = PCP_ERROR_QUIT + PCP_ERROR_UNAVAILABLE;
    pub const NO_HOST_OR_OFFAIR: u32 = PCP_ERROR_QUIT + PCP_ERROR_OFFAIR;
    pub const USER_SHUTDOWN: u32 = PCP_ERROR_QUIT + PCP_ERROR_SHUTDOWN;
}
/*
https://github.com/kimoto/peercast/blob/master/core/common/pcp.h
static const int PCP_ERROR_QUIT         = 1000;
static const int PCP_ERROR_BCST         = 2000;
static const int PCP_ERROR_READ         = 3000;
static const int PCP_ERROR_WRITE        = 4000;
static const int PCP_ERROR_GENERAL      = 5000;

static const int PCP_ERROR_SKIP             = 1;
static const int PCP_ERROR_ALREADYCONNECTED = 2;
static const int PCP_ERROR_UNAVAILABLE      = 3;
static const int PCP_ERROR_LOOPBACK         = 4;
static const int PCP_ERROR_NOTIDENTIFIED    = 5;
static const int PCP_ERROR_BADRESPONSE      = 6;
static const int PCP_ERROR_BADAGENT         = 7;
static const int PCP_ERROR_OFFAIR           = 8;
static const int PCP_ERROR_SHUTDOWN         = 9;
static const int PCP_ERROR_NOROOT           = 10;
static const int PCP_ERROR_BANNED           = 11;
 */
