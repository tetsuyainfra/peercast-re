///
///  PcpPing
/// example: cargo run --bin ping -- 192.168.0.1:7144
//  TODO: Rootサーバーに対してポートチェックしてもらう通信をおこなう機能を実装する
//  TODO: Tracker/Relayに対してポートチェックしてもらう通信をおこなう機能を実装する
//  MEMO: portcheckしてもらうにはHttpでChannelIdを通知する必要がある
use std::time::Duration;

use clap::Parser;
use libpeercast_re::pcp::{GnuId, PcpConnectionFactory};

#[derive(Parser, Debug)]
#[command(name = env!("CARGO_BIN_NAME"))]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(required = true)]
    pub ping_to: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    dbg!(&args);

    let factory = PcpConnectionFactory::builder(GnuId::new(), "0.0.0.0:7144".parse().unwrap())
        .connect_timeout(Duration::from_secs(1))
        .build();

    let socket = args
        .ping_to
        .parse()
        .expect("Ping先アドレスの分析に失敗しました");
    let handshake = factory.connect(socket).await?;
    dbg!(&handshake);

    let ping_fut = handshake.ping();

    match ping_fut.await {
        Ok(id) => println!("{socket}へのPingが 成功 しました. RemoteGnuID: {id}"),
        Err(_) => println!("{socket}へのPingが 失敗 しました"),
    };

    Ok(())
}
