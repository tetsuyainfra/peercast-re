use peercast_re::api::{router, ReStore};



fn main() {
    let (_,  api ) = router(ReStore{}.into());

    let j = api.to_pretty_json().unwrap();
    println!("{j}");

}
