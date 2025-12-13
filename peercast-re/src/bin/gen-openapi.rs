use peercast_re::{peercast::Store, handler::api::build_api};

fn main() {
    let (_, api) = build_api(Store::default().into());

    let j = api.to_pretty_json().unwrap();
    println!("{j}");
}
