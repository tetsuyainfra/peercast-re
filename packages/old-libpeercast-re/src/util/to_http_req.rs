use clap::builder;
use http::{HeaderMap, HeaderName, HeaderValue, Method, Uri, Version};

pub fn to_http_request(req: &httparse::Request<'_, '_>) -> Result<http::Request<()>, Box<dyn std::error::Error>> {
    // method
    let method = Method::from_bytes(req.method.ok_or("no method")?.as_bytes())?;

    // path
    // MEMO: OPTIONSメソッドの場合、pathがない可能性がある。しかしサーバー全体を表す/と等価なのでunwrap可能
    let uri: Uri = req.path.unwrap_or("").parse()?;

    // version
    let version = match req.version {
        Some(0) => Version::HTTP_10,
        Some(1) => Version::HTTP_11,
        _ => return Err("unsupported http version".into()),
    };

    // headers
    let mut builder = http::Request::builder().method(method).uri(uri).version(version);
    {
        let headers = builder.headers_mut().unwrap();
        for h in req.headers.iter() {
            headers.insert(HeaderName::from_bytes(h.name.as_bytes())?, HeaderValue::from_bytes(h.value)?);
        }
    }

    Ok(builder.body(())?)
}
