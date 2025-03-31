use std::fmt::Write;

use bytes::{Buf, BufMut, BytesMut};
use http::{Request, StatusCode, Version};
use tracing::debug;

use crate::pcp::GnuId;

pub(super) struct RequestHead {
    method: http::Method,
    uri: http::Uri,
    version: http::Version,
    headers: http::HeaderMap,
}

impl RequestHead {
    pub(super) fn new(value: http::request::Parts) -> Self {
        RequestHead {
            method: value.method,
            uri: value.uri,
            version: value.version,
            headers: value.headers,
        }
    }
}

impl From<RequestHead> for bytes::BytesMut {
    fn from(value: RequestHead) -> Self {
        let mut buf = BytesMut::with_capacity(1024);
        buf.write_fmt(format_args!(
            "{} {} {:?}\r\n",
            value.method, value.uri, value.version
        ))
        .unwrap();
        for (k, v) in value.headers.iter() {
            buf.write_str(k.as_str()).unwrap();
            buf.write_str(": ").unwrap();
            buf.put_slice(v.as_bytes());
            buf.write_str("\r\n").unwrap();
        }
        buf.write_str("\r\n").unwrap();
        buf
    }
}

pub(super) fn create_channel_request(broadcast_id: GnuId) -> BytesMut {
    let req = Request::builder()
        .method("GET")
        .uri(format!("/channel/{}", broadcast_id))
        .header("x-peercast-pcp", "1")
        .body(())
        .unwrap();

    let (parts, body) = req.into_parts();
    let mut req_buf: BytesMut = RequestHead::new(parts).into();

    return req_buf;
}

///
/// Result<Option<(http::Response<()>, usize)>, httparse::Error>
/// 返り値のusizeは読み込んだバッファーのサイズ
///
pub(super) fn parse_pcp_http_response(
    buf: &[u8],
) -> Result<Option<(http::Response<()>, usize)>, httparse::Error> {
    let mut parsed_headers = [httparse::EMPTY_HEADER; 64];
    let mut response = httparse::Response::new(&mut parsed_headers);

    match response.parse(&buf) {
        Ok(httparse::Status::Partial) => Ok(None),
        Ok(httparse::Status::Complete(header_bytes_len)) => {
            let status = StatusCode::from_u16(response.code.unwrap())
                .map_err(|e| httparse::Error::Status)?;

            let version = if response.version.unwrap() == 0 {
                Version::HTTP_10
            } else {
                Version::HTTP_11
            };

            let builder = http::Response::builder().status(status).version(version);

            let builder = response
                .headers
                .iter()
                .fold(builder, |b, header| b.header(header.name, header.value));

            let resp = builder.body(()).unwrap();
            Ok(Some((resp, header_bytes_len)))
        }
        Err(e) => Err(e.into()),
    }
}
