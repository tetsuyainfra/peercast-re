use std::net::{Ipv4Addr, SocketAddrV4};

use clap::Parser;
use peercast_re::pcp::{GnuId, PcpConnectionFactory};

#[derive(Parser, Debug)]
#[command(name = env!("CARGO_BIN_NAME"))]
#[command(version, about, long_about = None)]
pub struct Args {
    /// PeerCast root server address
    // #[arg(short, long, default_value = "0.0.0.0")]
    // pub bind: std::net::IpAddr,

    // #[cfg(not(debug_assertions))]
    // /// PeerCast root server port
    // #[arg(short, long, default_value_t = 7144)]
    // pub port: u16,

    #[arg(required = true)]
    pub ping_to: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let factory = PcpConnectionFactory::new(GnuId::new());

    let socket = args.ping_to.parse().unwrap();
    let handshake = factory.connect(socket).await?;
    // handshake.ping_pong_check().await;
    dbg!(handshake);

    Ok(())
}
