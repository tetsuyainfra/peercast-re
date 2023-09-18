#![allow(dead_code)]

use std::str::from_utf8;

use thiserror::Error;
use tokio::net::TcpStream;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionProtocol {
    PeerCast,
    PeerCastHttp,
    Http,
    Unknown,
}

#[derive(Debug, Error)]
pub enum IdentifierError {
    #[error("failed to identify protocol")]
    ProtocolIdentifierFailed,
    #[error("connection disconnect")]
    IOError(#[from] std::io::Error),
    #[error("parse failed")]
    HttpParseError(#[from] httparse::Error),
}

pub async fn identify_protocol(stream: &TcpStream) -> Result<ConnectionProtocol, IdentifierError> {
    let mut last_read = 0;
    let mut buf = [0_u8; 8192];

    let conn_type = loop {
        // tracing::log::info!("loop");
        // MEMO: curl で POSTした時に判定ができない
        let n = stream.peek(&mut buf).await?;
        match n {
            0 => {
                // connection closed?
                return Ok(ConnectionProtocol::Unknown);
            }
            _ => {}
        }

        if last_read != n {
            tracing::log::debug!("check: {}bytes", n);
            tracing::log::trace!("check: {:?}", from_utf8(&buf[..n]));
            last_read = n;
            match _identify_protocol(&buf, last_read) {
                Some(proto) => break proto,
                None => continue,
            };
        } else if n == buf.len() {
            // bail!("ProtoclCheck中に確保しているバッファの最大まで使用したが、判別できなかった")
            return Err(IdentifierError::ProtocolIdentifierFailed);
        }
    };
    tracing::log::debug!("ConnectionProto {:?}", &conn_type);
    // tracing::log::trace!("buffer {:?}", &buf[..last_read]);
    Ok(conn_type)
}

#[inline]
fn _identify_protocol(buf: &[u8], length: usize) -> Option<ConnectionProtocol> {
    if is_pcp(buf, length) {
        return Some(ConnectionProtocol::PeerCast);
    }

    return http_type(buf, length);
}

#[inline]
fn is_pcp(buf: &[u8], length: usize) -> bool {
    if length < 4 {
        return false;
    }
    &buf[0..4] == b"pcp\n"
}

const PCP_HEADER: &[u8; 14] = b"x-peercast-pcp";

#[inline]
fn http_type(buf: &[u8], length: usize) -> Option<ConnectionProtocol> {
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut req = httparse::Request::new(&mut headers);

    let status = req.parse(&buf[..length]);
    if status.is_err() {
        return Some(ConnectionProtocol::Unknown);
    }
    let is_request_completed: bool = status.unwrap().is_complete();

    // method, path, version, headerと見ていき、判定する
    // 何もなければ分からない場合Noneを返す
    let method = req.method?;
    let path = req.path?;
    let have_pcp_header: bool = req
        .headers
        .iter()
        .any(|h| h.name == "x-peercast-pcp" && h.value == b"1");

    if method.to_uppercase() != "GET" {
        return Some(ConnectionProtocol::Http);
    }

    if !path.starts_with("/channel/") {
        return Some(ConnectionProtocol::Http);
    }

    if !have_pcp_header {
        if is_request_completed {
            return Some(ConnectionProtocol::Http);
        } else {
            return None;
        }
    }

    Some(ConnectionProtocol::PeerCastHttp)
}

#[cfg(test)]
mod t {
    // use tokio_test::io::Builder;
    use super::*;

    #[test]
    fn test_identity_protocol() {
        let buf = b"pcp\n1223345521";
        let x: Option<ConnectionProtocol> = _identify_protocol(buf, buf.len());
        assert_eq!(x, Some(ConnectionProtocol::PeerCast));

        let buf = b"GET /channel/1 HTTP/1.0\r\nx-peercast-pcp:1\r\n\r\n";
        let x = _identify_protocol(buf, buf.len());
        assert_eq!(x, Some(ConnectionProtocol::PeerCastHttp));

        let buf = b"get /channel/1 HTTP/1.0\r\nx-peercast-pcp:1\r\n\r\n";
        let x = _identify_protocol(buf, buf.len());
        assert_eq!(x, Some(ConnectionProtocol::PeerCastHttp));

        let buf = b"POST /channel/1 HTTP/1.0\r\nx-peercast-pcp:1\r\n\r\n";
        let x = _identify_protocol(buf, buf.len());
        assert_eq!(x, Some(ConnectionProtocol::Http));

        let buf = b"GET / HTTP/1.0\r\nx-peercast-pcp:1\r\n\r\n";
        let x = _identify_protocol(buf, buf.len());
        assert_eq!(x, Some(ConnectionProtocol::Http));

        let buf = b"";
        let x: Option<ConnectionProtocol> = _identify_protocol(buf, buf.len());
        assert_eq!(x, None);

        let buf = b" ";
        let x: Option<ConnectionProtocol> = _identify_protocol(buf, buf.len());
        assert_eq!(x, None);

        let buf = b"helo";
        let x: Option<ConnectionProtocol> = _identify_protocol(buf, buf.len());
        assert_eq!(x, None);

        let buf = b"g";
        let x: Option<ConnectionProtocol> = _identify_protocol(buf, buf.len());
        assert_eq!(x, None);
    }
}

#[cfg(test)]
mod t_othercrate {
    #[crate::test]
    async fn test_httpparse_complete() {
        let    buf = b"POST /ch HTTP/1.1\r\nHost: localhost:3000\r\nUser-Agent: curl/7.81.0\r\nAccept: */*\r\nContent-Length: 8\r\nContent-Type: application/x-www-form-urlencoded\r\n\r\nname=aaa";

        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);
        let s = req.parse(buf).unwrap();
        assert!(s.is_complete());
    }

    #[test]
    fn test_httpparse_uncomplete() {
        let buf = b"POST /ch HTTP/1.1\r\n";
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);
        assert!(req.parse(buf).unwrap().is_partial());

        let buf = b"GET /channels/1 HTTP/1.0\r\nx-peercast-pcp:1\r\n\r\n";
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);
        assert_eq!(req.parse(buf).unwrap().is_complete(), true);

        assert_eq!(
            req.headers
                .iter()
                .any(|h| { h.name == "x-peercast-pcp" && h.value == b"1" }),
            true
        );

        let buf = b"GET /channels/1 HTTP/1.0\r\nx-peercast-pcp:\r\n\r\n";
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);
        assert_eq!(req.parse(buf).unwrap().is_complete(), true);

        assert_eq!(
            req.headers
                .iter()
                .any(|h| { h.name == "x-peercast-pcp" && h.value == b"" }),
            true
        );
    }
    #[test]
    fn test_httpparse_partial() {
        let buf = b"get /ch HTTP/1.1";
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);
        let r = req.parse(buf).unwrap();
        assert!(r.is_partial());
        assert_eq!(req.method.unwrap(), "get");

        let buf = b"POST /ch HTTP/1.1";
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);
        let r = req.parse(buf).unwrap();
        assert!(r.is_partial());
        assert_eq!(req.method.unwrap(), "POST");

        let buf = b"POST ";
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);
        let r = req.parse(buf).unwrap();
        assert!(r.is_partial());
        assert_eq!(req.method.unwrap(), "POST");

        let buf = b"pcp\n";
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);
        let r = req.parse(buf);
        assert_eq!(r.is_err(), true);
        assert_eq!(r.unwrap_err(), httparse::Error::Token);

        let buf = b"g";
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);
        let r = req.parse(buf).unwrap();
        assert_eq!(r.is_partial(), true);
        assert_eq!(req.method.is_none(), true);
    }
}
