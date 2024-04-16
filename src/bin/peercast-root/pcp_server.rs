use std::sync::Arc;

use peercast_re::{
    error::HandshakeError,
    pcp::{decode::PcpBroadcast, GnuId, PcpConnectType, PcpConnectionFactory},
};
use tracing::{debug, error, info, trace, warn};

use crate::{
    channel::{ChannelStore, TrackerChannel, TrackerChannelConfig},
    connection::TrackerConnection,
};

////////////////////////////////////////////////////////////////////////////////
/// PCP Server
///
pub async fn start_pcp_server(
    arc_channel_store: Arc<ChannelStore<TrackerChannel>>,
    listener: tokio::net::TcpListener,
) {
    info!("START PCP SERVER");
    let self_session_id = GnuId::new();
    let factory = PcpConnectionFactory::new(self_session_id);

    loop {
        let channel_store = arc_channel_store.clone();
        let (stream, remote) = listener.accept().await.unwrap();
        let pcp_handshake = factory.accept(stream, remote);

        let _x: tokio::task::JoinHandle<Result<(), HandshakeError>> = tokio::spawn(async move {
            let mut pcp_connection = pcp_handshake.incoming_pcp().await?;
            println!("{:#?}", &pcp_connection);

            let tracker_connection: TrackerConnection = match &pcp_connection.con_type {
                PcpConnectType::Outgoing => unreachable!(),
                PcpConnectType::IncomingPing(_ping) => {
                    // pingを返す(まー必要ないはずなんだけどあり得る通信なので)
                    todo!("pongを返してタスクを終了する")
                }
                PcpConnectType::IncomingBroadcast(helo) => {
                    let session_id = Arc::new(helo.session_id.clone());
                    // BroadcastIdは配信時にRoot(YP)に送られるID、これを知っているのはRootとTrackerなので認証することができる
                    // とりあえずココではBroadcastIdが存在する事を保障する
                    let broadcast_id = Arc::new(helo.broadcast_id.clone());
                    let Some(broadcast_id) = helo.broadcast_id.map(|g| Arc::new(g)) else {
                        error!(
                            "Helo Atom must have BroadcastId CID: {}",
                            pcp_connection.connection_id()
                        );
                        debug!("first atom: {:#?}", helo);
                        return Err(HandshakeError::Failed);
                    };

                    // ChannelIDはHandshake後、最初のAtom(id=Broadcast)に入っているため、一つ目を必ず読み取らなければならない。
                    // これでどの配信チャンネルに対する配信情報の送信か決定できるようになる
                    let first_atom = pcp_connection.read_atom().await?;
                    let Ok(first_broadcast) = PcpBroadcast::parse(&first_atom).map(|p| Arc::new(p))
                    else {
                        error!(
                            "First PCPPacket must be BroadcastAtom(id=bcst): {}",
                            pcp_connection.connection_id()
                        );
                        debug!("first atom: {:#?}", first_atom);
                        return Err(HandshakeError::Failed);
                    };

                    let Some(channel_id) = first_broadcast.channel_id else {
                        error!("first Broadcast must have ChannelId");
                        debug!("first broadcast: {:#?}", first_broadcast);

                        return Err(HandshakeError::Failed);
                    };

                    // 新規作成する場合、HELOで渡されたBCIDを渡しておく
                    let channel = channel_store.create_or_get(
                        channel_id,
                        TrackerChannelConfig {
                            tracker_session_id: session_id,
                            tracker_broadcast_id: broadcast_id.clone(),
                            first_broadcast: first_broadcast.clone(),
                        },
                    );

                    // TrackerConnectionを返す
                    channel.tracker_connection(
                        pcp_connection.connection_id(),
                        broadcast_id,
                        first_broadcast,
                    )
                }
            };

            // TrackerConnectionManagerと接続開始する
            let _ = tracker_connection
                .start_connect_manager(pcp_connection)
                .await;

            Ok(())
        });
    }
}
