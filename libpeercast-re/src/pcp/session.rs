use std::{io::Write, sync::atomic::AtomicI32};

use bytes::{Buf, Bytes, BytesMut};
use rml_rtmp::sessions::ServerSessionEvent;
use thiserror::Error;
use tracing::{debug, info, instrument, log::warn, trace};

use crate::{
    error::AtomParseError,
    pcp::util::atom as _in,
    pcp::{atom, Atom, GnuId, Id4},
};

use super::{classify::ClassifyAtom, ChannelInfo, TrackInfo};

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("parse error")]
    Parse(#[from] AtomParseError),
}

//------------------------------------------------------------------------------
// SessionConfig
//
#[derive(Debug)]
pub struct SessionConfig {}
impl SessionConfig {
    pub fn new() -> Self {
        Self {}
    }
}

//------------------------------------------------------------------------------
// Session
//
#[derive(Debug)]
pub struct Session {
    bytes_received: u64,
    deserializer: AtomDeserializer,
    // file: std::fs::File,
    count: AtomicI32,
}

impl Session {
    pub fn new(config: SessionConfig) -> Self {
        Self {
            bytes_received: 0,
            deserializer: AtomDeserializer::new(),
            // file: std::fs::File::create("tmp/stream.out").unwrap(),
            count: AtomicI32::new(0),
        }
    }

    pub fn handle_input(&mut self, slice: &[u8]) -> Result<Vec<SessionResult>, SessionError> {
        let mut results = vec![];
        // let count = self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        // info!("file count: {count}");
        // let mut f = std::fs::File::create(format!("tmp/dump_{count}.out",)).unwrap();
        // f.write_all(slice.clone());

        let mut arrived_bytes = slice;
        loop {
            let ret = self.deserializer.get_next_atom(arrived_bytes);
            arrived_bytes = &[];

            match ret {
                Ok(None) => break,
                Ok(Some(atom)) => {
                    // Atomの中身見てタイプ分けして処理する
                    // let message_results = ;
                    let message_result = match ClassifyAtom::classify(atom) {
                        ClassifyAtom::ChanPktHead {
                            atom,
                            payload,
                            pos,
                            info,
                            track,
                        } => SessionResult::RaisedEvent(SessionEvent::ArrivedHeadData {
                            atom,
                            head_data: payload,
                            pos,
                            info,
                            track,
                        }),
                        ClassifyAtom::ChanPktData {
                            atom,
                            payload,
                            pos,
                            continuation,
                        } => SessionResult::RaisedEvent(SessionEvent::ArrivedData {
                            atom,
                            data: payload,
                            pos,
                            continuation,
                        }),
                        ClassifyAtom::Unknown { atom } => {
                            continue;
                        }
                    };

                    results.push(message_result)
                }
                Err(e) => return Err(e.into()),
            } //match
        } // loop
        Ok(results)
    }

    fn handle_metadata() {}
}

//------------------------------------------------------------------------------
// SessionResult
//
#[derive(Debug)]
pub enum SessionResult {
    OutboundResponse(Atom),
    RaisedEvent(SessionEvent),
    Unknown(Atom),
}

//------------------------------------------------------------------------------
// SessionEvent
//
pub enum SessionEvent {
    //
    ArrivedData {
        atom: Atom,
        data: Bytes,
        pos: u32,
        continuation: Option<bool>,
    },
    ArrivedHeadData {
        atom: Atom,
        head_data: Bytes,
        pos: u32,
        //
        info: Option<ChannelInfo>,
        track: Option<TrackInfo>,
    },
}

impl std::fmt::Debug for SessionEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ArrivedData {
                atom,
                data,
                pos,
                continuation,
            } => f
                .debug_struct("ArrivedData")
                // .field("atom", atom)
                // .field("data", data)
                .field("pos", pos)
                .field("continuation", continuation)
                .finish_non_exhaustive(),
            Self::ArrivedHeadData {
                atom,
                head_data,
                pos,
                info,
                track,
            } => f
                .debug_struct("ArrivedHeadData")
                // .field("atom", atom)
                // .field("head_data", head_data)
                .field("pos", pos)
                .field("info", info)
                .field("track", track)
                .finish_non_exhaustive(),
        }
    }
}

//------------------------------------------------------------------------------
// AtomDeserializer
//

#[derive(Debug)]
struct AtomDeserializer {
    buffer: BytesMut,
}

impl AtomDeserializer {
    fn new() -> Self {
        Self {
            buffer: BytesMut::new(),
        }
    }

    fn get_next_atom(&mut self, new_bytes: &[u8]) -> Result<Option<Atom>, AtomParseError> {
        self.buffer.extend_from_slice(new_bytes);

        match Atom::parse(&mut self.buffer) {
            Ok(atom) => Ok(Some(atom)),
            Err(e) => match e {
                AtomParseError::NotEnoughRecievedBuffer(_) => Ok(None),
                AtomParseError::Unknown => Err(e),
                AtomParseError::NotFoundValue => Err(e),
                AtomParseError::IdError => Err(e),
                AtomParseError::ValueError => Err(e),
            },
        }
    }
}
