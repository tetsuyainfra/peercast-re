///
///  PcpPing
/// example: cargo run --bin ping -- 192.168.0.1:7144
//  TODO: Rootサーバーに対してポートチェックしてもらう通信をおこなう機能を実装する
//  TODO: Tracker/Relayに対してポートチェックしてもらう通信をおこなう機能を実装する
//  MEMO: portcheckしてもらうにはHttpでChannelIdを通知する必要がある
use clap::Parser;
use futures_util::StreamExt;
use libpeercast_re::pcp::{AtomCodec, GnuId, procedure::OutgoingPcpHandshake};
use tokio::net::TcpListener;
use tokio_util::codec::Framed;

#[derive(Parser, Debug)]
#[command(name = env!("CARGO_BIN_NAME"))]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(long, short, default_value_t = 7144)]
    pub bind: u16,

    #[arg(long)]
    pub check_port: Option<u16>,

    #[arg(required = true)]
    pub ping_to: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    dbg!(&args);

    let self_session_id = GnuId::new();
    let bind = TcpListener::bind(format!("0.0.0.0:{}", args.bind)).await?;
    let _handle = tokio::spawn(server(self_session_id, bind));

    let remote = args.ping_to.parse().expect("Ping先アドレスの分析に失敗しました");
    let stream = tokio::net::TcpStream::connect(remote).await?;
    // let local_addr = stream.local_addr().unwrap();
    let handshake = OutgoingPcpHandshake::new(stream, remote, None);
    let (oleh, handshake) = handshake.ping(self_session_id, Some(args.bind), args.check_port).await?;

    dbg!(&oleh);
    dbg!(&handshake);

    Ok(())
}

async fn server(_self_session_id: GnuId, bind: TcpListener) -> anyhow::Result<()> {
    loop {
        let (stream, _remote) = bind.accept().await?;

        let mut framed = Framed::new(stream, AtomCodec::new());
        let atom = framed.next().await;
        dbg!(atom);

        // TODO: return Oleh
    }
}
