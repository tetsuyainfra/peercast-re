use std::net::SocketAddr;

use bytes::BytesMut;
use rml_rtmp::handshake;
use tokio::io::AsyncReadExt;

use crate::{
    io::Io,
    pcp::procedure::handshake::{
        HandshakeError, IncomingHttpHandshake, IncomingHttpPcpHandshake, IncomingPcpHandshake,
        IncomingUnknownHandshake, Parts,
    },
    util::identify2::{identify_protocol, ConnectionProtocol},
    ConnectionNo,
};

/// 着信したストリームのプロトコルを識別する型
pub struct IncomingProtocolDiscriminator<S: Io> {
    timeout: std::time::Duration,
    marker: std::marker::PhantomData<S>,
}

impl<S: Io> IncomingProtocolDiscriminator<S> {
    const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
    pub fn new() -> Self {
        Self {
            timeout: Self::TIMEOUT,
            marker: std::marker::PhantomData,
        }
    }

    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub async fn identify(
        self,
        cno: ConnectionNo,
        mut stream: S,
        remote: SocketAddr,
    ) -> Result<HandshakeProtocol<S>, HandshakeError> {
        let parts = Parts::new(cno, stream, remote, None, None);
        tokio::time::timeout(self.timeout, self._identify(parts)).await?
    }

    async fn _identify(self, mut parts: Parts<S>) -> Result<HandshakeProtocol<S>, HandshakeError> {
        loop {
            let n = parts.stream.read_buf(&mut parts.read_buf).await?;
            if n == 0 {
                break;
            }
            let protocol = identify_protocol(&mut parts.read_buf);

            if let Some(protocol) = protocol {
                match protocol {
                    ConnectionProtocol::Pcp => {
                        let handshake = IncomingPcpHandshake::from_parts(parts);
                        return Ok(HandshakeProtocol::Pcp(handshake));
                    }
                    ConnectionProtocol::HttpPcp => {
                        let handshake = IncomingHttpPcpHandshake::from_parts(parts);
                        return Ok(HandshakeProtocol::HttpPcp(handshake));
                    }
                    ConnectionProtocol::Http => {
                        let handshake = IncomingHttpHandshake::from_parts(parts);
                        return Ok(HandshakeProtocol::Http(handshake));
                    }
                    ConnectionProtocol::Unknown => {
                        let handshake = IncomingUnknownHandshake::from_parts(parts);
                        return Ok(HandshakeProtocol::Unknown(handshake));
                    }
                };
            } else {
                continue;
            };
        }

        // let handshake = IncomingUnknownHandshake::new(stream, remote, Some(buf));
        // Ok(HandshakeProtocol::Unknown(handshake));
        todo!()
    }
}

pub enum HandshakeProtocol<S: Io> {
    Pcp(IncomingPcpHandshake<S>),
    HttpPcp(IncomingHttpPcpHandshake<S>),
    Http(IncomingHttpHandshake<S>),
    Unknown(IncomingUnknownHandshake<S>),
}

#[cfg(test)]
mod t {
    use tokio::io::AsyncWriteExt;

    use crate::pcp::{Atom2, AtomMut, GnuId};

    use super::*;

    #[tokio::test]
    async fn test_identify_protocol() {
        let (mut server, mut client) = tokio::io::duplex(1024);
        let remote = "127.0.0.1:0".parse().unwrap();

        let buf = b"pcp\n\x04\x00\x00\x00\x01\x00\x00\x00";
        let x: Option<ConnectionProtocol> = identify_protocol(buf);
        assert_eq!(x, Some(ConnectionProtocol::Pcp));

        let r = client.write(buf).await;

        let cno = ConnectionNo::new();
        let prot = IncomingProtocolDiscriminator::new().identify(cno, server, remote).await;
        assert!(prot.is_ok());
        match prot.unwrap() {
            HandshakeProtocol::Pcp(handshake) => {
                handshake.handshake(GnuId::new()).await.unwrap();
            }
            _ => {
                assert!(false)
            }
        }
    }

    #[ignore = "longtime test"]
    #[tokio::test]
    async fn test_timeout() {
        let (mut server, mut client) = tokio::io::duplex(1024);
        let remote = "127.0.0.1:0".parse().unwrap();

        let cno = ConnectionNo::new();
        let prot = IncomingProtocolDiscriminator::new().identify(cno, server, remote).await;
        assert!(prot.is_err());

        match prot.err().unwrap() {
            HandshakeError::Timeout(elapsed) => assert!(true),
            _ => assert!(false),
        }
    }
}
