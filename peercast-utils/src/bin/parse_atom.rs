use futures_util::{StreamExt, TryStreamExt};
use libpeercast_renew::atom::{AtomView, codec::AtomCodec};
use tokio::fs::File;
use tokio_util::codec::Framed;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let filename = args.get(1).expect("need FILENAME");

    let file = File::open(filename).await.expect("file not found");

    let framed = Framed::new(file, AtomCodec::new());
    let _ = framed
        .try_for_each(|atom| async move {
            println!("Atom: kind={:?}, length={}, payload={:?}", atom.kind(), atom.length(), atom.payload());
            Ok(())
        })
        .await;
}
