fn main() {
    println!("PKG_VERSION: {}", peercast_re::PKG_VERSION);
    println!("PKG_VERSION_MAJOR: {}", peercast_re::PKG_VERSION_MAJOR);
    println!("PKG_VERSION_MINOR: {}", peercast_re::PKG_VERSION_MINOR);
    println!(
        "PKG_SERVANT_VERSION_EX_NUMBER: {}",
        *peercast_re::PKG_SERVANT_VERSION_EX_NUMBER
    );
}
