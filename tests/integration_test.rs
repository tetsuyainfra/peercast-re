extern crate peercast_re;

#[test]
fn it_version() {
    // assert_eq!(4, adder::add_two(2));
    assert_eq!(peercast_re::PKG_VERSION, "0.1.0");
}
