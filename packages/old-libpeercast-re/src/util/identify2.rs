use crate::{prelude::*, util::dump_str};
use bytes::BytesMut;
use minijinja::filters::last;
use nom::combinator::Opt;
use thiserror::Error;
use tokio::{io::AsyncRead, net::TcpStream};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionProtocol {
    /// PCP Protocol
    Pcp,
    /// HTTP then PCP Protocol
    HttpPcp,
    /// General HTTP Protocol
    Http,
    /// Unknown Protocol
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

// pub async fn identify_protocol2(stream: &mut TcpStream) -> Result<ConnectionProtocol, IdentifierError> {
//     let mut last_read = 0;
//     let mut buf = [0_u8; 8192];

//     loop {
//         let n = stream.peek(&mut buf).await?;
//         if (n == 0) {
//             // プロトコルが判定されないまま切断された
//             return Ok(ConnectionProtocol::Unknown);
//         }

//         // 前回と異なるバイト数が読めた場合のみ判定を試みる
//         // そして8192バイト読んだ時は一度は判定を試みる
//         if last_read != n {
//             trace!("check {}bytes: {}", n, dump_str(&buf[..n]));
//             last_read = n;

//             if let Some(prot) = identify_protocol(&buf, n) {
//                 return Ok(prot);
//             }
//         } else if n == buf.len() {
//             // ProtoclCheck中に確保しているバッファの最大まで使用したが、判別できなかった
//             error!("Unable to identify protocol within buffer limit");
//             return Err(IdentifierError::ProtocolIdentifierFailed);
//         }
//     }
// }

pub fn identify_protocol(buf: &[u8]) -> Option<ConnectionProtocol> {
    if is_pcp(buf) {
        return Some(ConnectionProtocol::Pcp);
    }
    http_type(buf)
}

#[inline]
fn is_pcp(buf: &[u8]) -> bool {
    if buf.len() < 4 {
        return false;
    }
    &buf[0..4] == b"pcp\n"
}

const PCP_HEADER: &[u8; 14] = b"x-peercast-pcp";

#[inline]
fn http_type(buf: &[u8]) -> Option<ConnectionProtocol> {
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut req = httparse::Request::new(&mut headers);

    let status = req.parse(&buf[..]);
    if status.is_err() {
        return Some(ConnectionProtocol::Unknown);
    }
    let is_request_completed: bool = status.unwrap().is_complete();
    let method = req.method?;
    let path = req.path?;
    let have_pcp_header: bool = req.headers.iter().any(|h| h.name == "x-peercast-pcp" && h.value == b"1");

    // PeercastのHTTPリクエストは次の形で送られてくる
    // GET /channel/1 HTTP/1.0\r\nother-header: hoge\r\nx-peercast-pcp: 1\r\n\r\n

    // つまりGET以外のリクエストはHTTPプロトコルとみなす
    if method.to_uppercase() != "GET" {
        return Some(ConnectionProtocol::Http);
    }

    // また、GETであっても、パスが/channel/で始まらないものはHTTPプロトコルとみなす
    if path.starts_with("/channel/") == false {
        return Some(ConnectionProtocol::Http);
    }

    // さらに、x-peercast-pcp:1 ヘッダがない場合もHTTPプロトコルとみなす
    // その場合、HTTPリクエストとしては完全に受信できている必要がある。
    // そうでない場合は、まだプロトコルを判定できないのでNoneを返す
    if !have_pcp_header {
        if is_request_completed {
            return Some(ConnectionProtocol::Http);
        } else {
            return None;
        }
    }

    // ここまで来たら、HTTP(GET /channel/であって、x-peercast-pcp:1 ヘッダもあることがわかる
    Some(ConnectionProtocol::HttpPcp)
}

#[async_trait::async_trait]
pub trait AsyncPeek {
    async fn peek(&self, buf: &mut [u8]) -> std::io::Result<usize>;
}

#[async_trait::async_trait]
impl AsyncPeek for tokio::net::TcpStream {
    async fn peek(&self, buf: &mut [u8]) -> std::io::Result<usize> {
        TcpStream::peek(self, buf).await
    }
}

#[cfg(test)]
mod t {
    // use tokio_test::io::Builder;
    use super::*;
    use tokio::{io::AsyncWriteExt, time};

    /// TCPStreamはOS依存なので、バッファを使ってテストする
    #[test]
    fn test_identity_protocol() {
        let buf = b"pcp\n1223345521";
        let x: Option<ConnectionProtocol> = identify_protocol(buf);
        assert_eq!(x, Some(ConnectionProtocol::Pcp));

        let buf = b"GET /channel/1 HTTP/1.0\r\nx-peercast-pcp:1\r\n\r\n";
        let x = identify_protocol(buf);
        assert_eq!(x, Some(ConnectionProtocol::HttpPcp));

        let buf = b"get /channel/1 HTTP/1.0\r\nx-peercast-pcp:1\r\n\r\n";
        let x = identify_protocol(buf);
        assert_eq!(x, Some(ConnectionProtocol::HttpPcp));

        let buf = b"POST /channel/1 HTTP/1.0\r\nx-peercast-pcp:1\r\n\r\n";
        let x = identify_protocol(buf);
        assert_eq!(x, Some(ConnectionProtocol::Http));
        let buf = b"POST /channel/1 HTTP/1.0\r\nx-peercast-pcp: 1\r\n\r\n";
        let x = identify_protocol(buf);
        assert_eq!(x, Some(ConnectionProtocol::Http));

        // peercast-pcpヘッダの送信は終わっているが, リクエスト全体が終わっていない場合
        let buf = b"GET /channel/1 HTTP/1.0\r\nx-peercast-pcp: 1\r\n";
        let x: Option<ConnectionProtocol> = identify_protocol(buf);
        assert_eq!(x, Some(ConnectionProtocol::HttpPcp));
        let buf = b"get /channel/1 HTTP/1.0\r\nx-peercast-pcp:1\r\n";
        let x = identify_protocol(buf);
        assert_eq!(x, Some(ConnectionProtocol::HttpPcp));

        // peercast-pcpヘッダの送信は終わっているが, リクエスト全体が終わっていない場合(ヘッダ値が空白)
        let buf = b"GET /channel/1 HTTP/1.0\r\nx-peercast-pcp: \r\n";
        let x: Option<ConnectionProtocol> = identify_protocol(buf);
        assert_eq!(x, None); // ヘッダーリクエストが全部終わるのを待つためNoneを返す

        let buf = b"GET / HTTP/1.0\r\nx-peercast-pcp:1\r\n\r\n";
        let x = identify_protocol(buf);
        assert_eq!(x, Some(ConnectionProtocol::Http));

        let buf = b"";
        let x: Option<ConnectionProtocol> = identify_protocol(buf);
        assert_eq!(x, None);

        let buf = b" ";
        let x: Option<ConnectionProtocol> = identify_protocol(buf);
        assert_eq!(x, Some(ConnectionProtocol::Unknown));

        let buf = b"helo";
        let x: Option<ConnectionProtocol> = identify_protocol(buf);
        assert_eq!(x, None);
    }
}

/// 外部クレートの動作確認用テスト
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

        assert_eq!(req.headers.iter().any(|h| { h.name == "x-peercast-pcp" && h.value == b"1" }), true);

        let buf = b"GET /channels/1 HTTP/1.0\r\nx-peercast-pcp:\r\n\r\n";
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);
        assert_eq!(req.parse(buf).unwrap().is_complete(), true);

        assert_eq!(req.headers.iter().any(|h| { h.name == "x-peercast-pcp" && h.value == b"" }), true);
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
