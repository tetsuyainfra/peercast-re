use anyhow::Ok;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let read_buf = b"GET / HTTP/1.1\r\nHost: example.com\r\n\r\nabcdefg";

    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut req = httparse::Request::new(&mut headers);
    let r = req.parse(&read_buf[..])?;
    dbg!(r);
    dbg!(req);
    dbg!(&read_buf[37..]);

    Ok(())
}
