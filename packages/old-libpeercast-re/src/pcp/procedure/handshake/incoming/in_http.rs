use std::{net::SocketAddr, time::Duration};

use bytes::{Buf, BytesMut};
use futures_util::{stream, StreamExt};
use http::request;
use tokio::{io::AsyncReadExt, net::TcpStream, time::timeout};
use tokio_util::codec::{Framed, FramedParts};

use crate::{
    io::Io,
    pcp::{
        builder2::OlehInfo,
        procedure::handshake::{HandshakeError, Parts},
        Atom, Atom2, AtomCodec,
    },
    util::to_http_request,
    ConnectionNo,
};

const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
////////////////////////////////////////////////////////////////////////////////
// Http
//

// HTTP requesのパース結果
#[derive(Debug)]
pub struct ResultHttpHandshake<S: Io> {
    pub request: request::Request<()>,
    pub parts: Parts<S>,
}

pub struct IncomingHttpHandshake<S: Io> {
    parts: Parts<S>,
    timeout: std::time::Duration,
}

impl<S: Io> IncomingHttpHandshake<S> {
    pub fn new(
        cno: ConnectionNo,
        stream: S,
        remote: SocketAddr,
        read_buf: Option<BytesMut>,
        write_buf: Option<BytesMut>,
    ) -> Self {
        Self {
            parts: Parts::new(cno, stream, remote, read_buf, write_buf),
            timeout: TIMEOUT.clone(),
        }
    }
    pub fn from_parts(parts: Parts<S>) -> Self {
        Self {
            parts,
            timeout: TIMEOUT.clone(),
        }
    }
    pub fn into_parts(self) -> Parts<S> {
        let Self {
            parts,
            timeout,
        } = self;
        parts
    }

    pub async fn shutdown(mut self) -> () {
        self.parts.stream.shutdown().await;
    }

    pub async fn handshake(self) -> Result<ResultHttpHandshake<S>, HandshakeError> {
        let Self {
            mut parts,
            timeout,
        } = self;

        // // 8192確保されていることを保証する
        // let size = read_buf.capacity() + read_buf.len();
        // if size < 8192 {
        //     read_buf.reserve(8192 - read_buf.len());
        // }

        let request = tokio::time::timeout(timeout, Self::parse_http_request(&mut parts)).await??;

        Ok(ResultHttpHandshake {
            request,
            parts,
        })
    }

    async fn parse_http_request(parts: &mut Parts<S>) -> Result<http::Request<()>, HandshakeError> {
        let (read_size, request) = loop {
            let mut headers = [httparse::EMPTY_HEADER; 64];
            let mut req = httparse::Request::new(&mut headers);

            // HTTPリクエストの解析
            let status = req.parse(&parts.read_buf).map_err(|_| HandshakeError::Failed)?;

            if status.is_complete() {
                let read_size = status.unwrap();
                let req = to_http_request(&req).map_err(|_| HandshakeError::Failed)?;
                break (read_size, req);
            } else {
                parts.stream.read_buf(&mut parts.read_buf).await;
            };
        };
        parts.read_buf.advance(read_size);

        Ok(request)
    }
}

#[cfg(test)]
mod t {
    use tokio::{io::AsyncWriteExt, net::TcpListener};

    use crate::pcp::{Atom2, AtomMut};

    use super::*;

    #[tokio::test]
    async fn test_identify_http() {
        let mut server = TcpListener::bind("127.0.0.0:0").await.unwrap();
        let remote = server.local_addr().unwrap();

        let _h = tokio::spawn(async move {
            let (mut stream, remote) = server.accept().await.unwrap();
            let mut buf = BytesMut::with_capacity(4096);
            stream.read_buf(&mut buf).await;

            let cno = ConnectionNo::new();
            let handshake = IncomingHttpHandshake::new(cno, stream, remote, buf.into(), None);
            let r = handshake.handshake().await.unwrap();
            dbg!(&r);
        });

        let mut client = TcpStream::connect(remote).await.unwrap();
        let buf = b"GET /stream/abcdefg HTTP/1.1\r\n";
        client.write(buf).await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        let buf2 = b"Host: example.com\r\n\r\nabcdefg";
        client.write(buf2).await;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Handshakeを実行する時点でバッファーにHTTPがある程度解析できる程度のデータが来ている場合
    #[tokio::test]
    async fn test_identify_http_test2() {
        let mut server = TcpListener::bind("127.0.0.0:0").await.unwrap();
        let remote = server.local_addr().unwrap();

        let _h = tokio::spawn(async move {
            let buf2 = b"XYZ";
            let mut client = TcpStream::connect(remote).await.unwrap();
            client.write(buf2).await;
            // tokio::time::sleep(Duration::from_millis(100)).await;
        });

        let (mut stream, remote) = server.accept().await.unwrap();
        let mut read_buf = BytesMut::with_capacity(4096);
        let buf = b"GET /stream/abcdefg HTTP/1.1\r\nHost: example.com\r\n\r\nabcdefg";
        read_buf.extend_from_slice(&buf[..]);

        let cno = ConnectionNo::new();
        let handshake = IncomingHttpHandshake::new(cno, stream, remote, read_buf.into(), None);
        let r = handshake.handshake().await.unwrap();
        dbg!(&r);
    }

    #[tokio::test]
    async fn test_identify_protocol() {
        let (mut server, mut client) = tokio::io::duplex(1024);
        let remote = "127.0.0.1:0".parse().unwrap();
        let buf = b"GET /stream/abcdefg HTTP/1.1\r\nHost: example.com\r\n\r\nabcdefg";
        let r = client.write(buf).await;
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            // client.write(b"xyz").await;
        });

        let mut read_buf = BytesMut::with_capacity(4096);
        server.read_buf(&mut read_buf).await;
        let cno = ConnectionNo::new();
        let handshake = IncomingHttpHandshake::new(cno, server, remote, Some(read_buf), None);
        let r = handshake.handshake().await.unwrap();
        dbg!(&r);
    }
}
