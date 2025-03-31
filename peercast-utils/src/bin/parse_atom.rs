use bytes::BytesMut;
use libpeercast_re::pcp::read_atom;
use tokio::fs::File;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let filename = args.get(1).expect("need FILENAME");

    let mut file = File::open(filename).await.expect("file not found");

    let mut buf = BytesMut::new();
    let atom = read_atom(&mut file, &mut buf).await;

    println!("{:#?}", atom);
}
