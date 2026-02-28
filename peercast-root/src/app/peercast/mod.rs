use std::net::SocketAddr;

use libpeercast_re::{
    ConnectionNo,
    pcp::connection5::{Connection, ConnectionFactory, ConnectionHandle, ConnectionManager, HandshakeConnection},
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::watch,
};
use tokio_util::sync::CancellationToken;

use crate::app::ArcState;
use peercast_root::{connection::RootHandshakeResult, prelude::*};

pub async fn server_peercast(
    state: ArcState,
    listener: TcpListener,
    graceful_shutdown: CancellationToken,
) -> anyhow::Result<()> {
    // スレッドの終了を検知するためのチャンネル
    let (closed_tx, closed_rx) = tokio::sync::watch::channel(());
    let tracker = tokio_util::task::TaskTracker::new();
    info!("START PCP SERVER");

    'accept: loop {
        let cid = ConnectionNo::new();
        let name = format!("tcp({})", cid);
        let spawner = tokio::task::Builder::new().name(&name);
        let child_graceful_shutdown = graceful_shutdown.child_token();
        let state = state.clone();

        // Dropすることで、全ての接続終了を確認する
        let closed_rx = closed_rx.clone();

        tokio::select! {
            accept = listener.accept() => {
                println!("accepting connection");
                match accept {
                    Ok((stream, addr)) => {
                        println!("{}: accept connection from {}", &name, &addr);
                        let _handle = spawner.spawn(tracker.track_future(serve_peercast(state, cid, stream, addr, child_graceful_shutdown, closed_rx.clone())));
                    }
                    Err(e) => {
                        error!(?e, "something is occured in listener.accept()");
                        break 'accept;
                    }
                }
            },
            _ = graceful_shutdown.cancelled() => {
                info!("GRACEFUL SHUTDOWN REQUESTED");
                break 'accept;
            }
        };
    }

    // 自身で持っている接続を閉じる
    drop(closed_rx);
    // 全ての接続が閉じるまで待つ
    let _ = closed_tx.closed().await;

    // trackerを閉じて、新規にSpawnできないようにし、全てのスレッドが終了するのを待つ
    tracker.close();
    tracker.wait().await;

    Ok(())
}

#[allow(unused)]
async fn serve_peercast(
    state: ArcState,
    cno: ConnectionNo,
    stream: TcpStream,
    remote: SocketAddr,
    graceful_shutdown: CancellationToken,
    closed_send: watch::Receiver<()>,
) -> anyhow::Result<()> {
    info!(?cno, ?remote, "SPAWN SERVE");
    let handshake_connection =
        state.connection_factory.create_accepted_connection(cno, remote, Some(graceful_shutdown.child_token()), None);

    let handshake_result = handshake_connection.handshake().await?;
    match handshake_result {
        RootHandshakeResult::Pcp(root_established) => {
            info!(?cno, ?remote, "Connection is PCP");
            let handle = root_established.handle();
            let task_name = handle.task_name();
            let span = tracing::info_span!("Conn", ?cno);
            let _ = tokio::task::Builder::new().name(&task_name).spawn(async move {
                let _enter = span.enter();
                root_established.run().await
            })?;
            state.connection_manager.insert(handle);
            Ok(())
        }
        _ => {
            info!(?cno, ?remote, "Connection Finished");
            Ok(())
        }
    }
}

/*
let discrimer = IncomingProtocolDiscriminator::new();

let handshake_protocol = discrimer.identify(cno, stream, remote).await?;
match handshake_protocol {
    handshake::HandshakeProtocol::Pcp(incoming_pcp_handshake) => {
        info!(?cno, ?remote, "ACCEPT PCP");
        let mut handshaked = incoming_pcp_handshake.handshake(state.self_session_id).await?;
        match handshaked.in_type {
            IncommingType::Ping => {
                info!(?cno, ?remote, "PCP PING shutting down");
                handshaked.shutdown().await;
                Ok(())
            }
            IncommingType::YellowPage(broadcast_id) => {
                serve_root(state, graceful_shutdown, broadcast_id, handshaked.parts).await
            }
        }
    }
    handshake::HandshakeProtocol::HttpPcp(incoming_http_pcp_handshake) => {
        info!(?cno, ?remote, "ACCEPT HTTP_PCP");
        incoming_http_pcp_handshake.shutdown().await;
        Ok(())
    }
    handshake::HandshakeProtocol::Http(incoming_http_handshake) => {
        info!(?cno, ?remote, "ACCEPT HTTP");
        incoming_http_handshake.shutdown().await;
        Ok(())
    }
    handshake::HandshakeProtocol::Unknown(incoming_unknown_handshake) => {
        info!(?cno, ?remote, "ACCEPT Unknown");
        incoming_unknown_handshake.shutdown().await;
        Ok(())
    }
} */
/*
//-------------------------------------------------------------------------------
// PCP
//-------------------------------------------------------------------------------
async fn serve_root<S: Io>(
    state: ArcState,
    graceful_shutdown: CancellationToken,
    broadcast_id: GnuId,
    parts: PartsWrapFramed<S>,
) -> anyhow::Result<()> {
    info!(?parts.cno, ?parts.remote, "serve_root");
    let PartsWrapFramed {
        cno,
        remote,
        mut framed,
    } = parts;
    // PCP_ROOTを送る
    // が、実際のところYTでもStでも無視してる
    let root_atom = RootBuilder2::default().build();
    let _ = framed.send(root_atom).await?;

    // PCP_OKを送る
    // が、実際のところYTでもStでも無視してる
    let ok_atom = OkBuilder2::new(0).build();
    let _ = framed.send(ok_atom).await?;

    // Updateを送る
    let root_atom = RootBuilder2::build_update_request();
    let _ = framed.send(root_atom).await?;

    // Braodcastを待つ
    let Some(Ok(broadcast_candidate)) = framed.next().await else {
        return Err(anyhow!("failed to receive broadcast"));
    };
    let broadcast = BroadcastInfo::try_from(&broadcast_candidate)?;
    let BroadcastInfo {
        from_session_id,
        channel_id,
        broadcast_group,
        chan,
        host,
        ..
    } = broadcast;
    let Some(chan) = chan else {
        return Err(anyhow!("failed to receive channel"));
    };

    // IDチェック
    let Some(channel_id) = channel_id else {
        return Err(anyhow!("failed to receive channel_id"));
    };
    let Some(chan_channel_id) = chan.broadcast_id else {
        return Err(anyhow!("failed to receive channel_id"));
    };
    let Some(chan_broadcast_id) = chan.broadcast_id else {
        return Err(anyhow!("failed to receive broadcast_id"));
    };
    if broadcast_id != chan_broadcast_id || channel_id != chan_channel_id {
        return Err(anyhow!("Suspcious broadcast_id or channel_id"));
    }

    let Some(channel_info) = chan.channel_info else {
        return Err(anyhow!("failed to receive channel_info"));
    };
    let Some(track_info) = chan.track_info else {
        return Err(anyhow!("failed to receive track_info"));
    };

    let valid_channel_info = ValidChannelInfo::from(&channel_info);
    let valid_track_info = ValidTrackInfo::from(&track_info);

    let target_channel =
        state.repository.create_or_get(channel_id, Some(valid_channel_info), Some(valid_track_info), None).await;

    let channel_ctrl = target_channel.control_with_authenticate(broadcast_id);

    // state.connection_factory.create_accepted_connection(cno, stream, remote, write_buf, read_buf).await;

    // ここでいよいよConnectionにしてWrapする
    // let name = format!("PCP-{}", handshaked.parts.cno);
    // let spawner = tokio::task::Builder::new().name(&name);
    // let _handle = spawner.spawn(serve_root(
    //     state,
    //     graceful_shutdown,
    //     name.clone(),
    //     broadcast_id,
    //     handshaked.parts,
    // ))?;
    // {}

    Ok(())
}
    */

/*
#[inline]
async fn serve_root(
    state: ArcState,
    cid: ConnectionNo,
    stream: TcpStream,
    remote: SocketAddr,
    graceful_shutdown: CancellationToken,
    closed_send: watch::Receiver<()>,
) {
    use libpeercast_re::pcp::connection::HandshakeType;

    let _read_buf = BytesMut::new();

    // HandshakeFutureにすればよさそう
    let handshake = state.0.conn_factory.accept(cid, stream, remote);

    // Handshake時に送ってもらうAtomを作成する
    let root_atom = RootBuilder::default().set_update_interval(10).set_next_update_interval(10).build();

    let mut conn = match handshake.incoming(root_atom.into()).await {
        Err(_e) => {
            todo!();
            return;
        }
        Ok(HandshakeType::Ping) => return,
        Ok(HandshakeType::YellowPage(conn)) => conn,
    };

    // RootならTrackerに次の情報を送って、情報のアップデートを求める(Broadcastを遅らせる)
    let root_atom = RootBuilder::build_update_request();
    conn.write_atom(root_atom).await;

    // 最初のAtomはBroadcastが確定する
    let first_atom = match conn.read_atom().await {
        Ok(a) => a,
        Err(_) => return,
    };
    dbg!(&first_atom);

    let bcst = match PcpBroadcast::parse(&first_atom) {
        Ok(b) => b,
        Err(_) => return,
    };
    dbg!(&bcst);

    // パケットの中身が適正か確認する
    let PcpBroadcast {
        channel_id,
        channel_packet,
        host,
        ..
    } = &bcst;
    let (channel_id_in_bcst, channel_packet) = match (channel_id, channel_packet) {
        (Some(chid), Some(chpkt)) => (chid, chpkt),
        _ => return,
    };
    // TODO: HostのIPチェックを行う？

    // Hostの接続先を確定
    let tracker_host = host.as_ref().and_then(|pcp_host| get_tracker_addr(&remote, &pcp_host.addresses));

    let PcpChannel {
        channel_id,
        broadcast_id,
        channel_info,
        track_info,
        ..
    } = channel_packet;

    let (channel_id_in_chpkt, _braodcast_id) = match (channel_id, broadcast_id) {
        (Some(chid), Some(bcid)) => (chid, bcid),
        _ => return,
    };

    // 不正チェック
    if channel_id_in_bcst != channel_id_in_chpkt {
        return;
    }

    // チャンネル情報の変換
    let channel_info = channel_info.as_ref().map(|i| i.into());
    let track_info = track_info.as_ref().map(|t| t.into());
    //
    let config = RootConfig {
        // tracker_host,
    };

    // 対象チャンネルを取得
    let mut repo = &state.0.repository2;
    let ch = repo.create_or_get(*channel_id_in_bcst, channel_info, track_info, Some(config)).await;

    // Channelにコネクションを接続
    let attach_task = ch.attach_connection(conn, graceful_shutdown, closed_send);
    attach_task.await;
}

fn get_tracker_addr(remote_addr: &SocketAddr, addresses: &Vec<SocketAddr>) -> Option<SocketAddr> {
    // // Hostの接続先を確定
    // // TODO: firewall checkが必要
    // let host = addresses.iter().find(|addr| addr.ip() == remote_addr.ip());
    // let tracker_host = host.map(|h| h.clone());

    // tracker_host
    todo!()
}
async fn check_protocol(
    cno: ConnectionNo,
    mut stream: TcpStream,
    remote: SocketAddr,
) -> Result<(TcpStream, BytesMut, ConnectionProtocol), ()> {
    const MAX_READ_SIZE: usize = 8192;
    let mut read_buff = BytesMut::with_capacity(MAX_READ_SIZE);

    let mut sum = 0;
    loop {
        if sum > MAX_READ_SIZE {
            error!(?cno, ?remote, "Read Buffer reach MAX_READ_SIZE");
            return Err(());
        }

        match stream.read_buf(&mut read_buff).await {
            Err(e) => {
                error!(?cno, ?remote, "Failed: read_buf: {}", e);
                return Err(());
            }
            Ok(0) => {
                info!(?cno, ?remote, "STREAM is closed");
                return Err(());
            }
            Ok(read_num) => {
                sum = sum + read_num;
            }
        };

        match identify_protocol(&read_buff[..]) {
            Some(protocol) => return Ok((stream, read_buff, protocol)),
            None => continue,
        }
    }
}
    // let (mut stream, buff, protocol) = match timeout(Duration::from_secs(5), check_protocol(cno, stream, remote)).await
    // {
    //     Err(e) => {
    //         error!(?cno, ?remote, "Failed: timeout: {}", e);
    //         // MEMO: stream.shutdown().await; は runtimeに任せる
    //         return;
    //     }
    //     Ok(Err(_e)) => {
    //         error!(?cno, ?remote, "Failed: check_protocol()");
    //         // MEMO: stream.shutdown().await; は runtimeに任せる
    //         return;
    //     }
    //     Ok(Ok(val)) => val,
    // };

    // match protocol {
    //     ConnectionProtocol::Pcp => {
    //         info!(?cno, ?remote, "STREAM is PeerCast Protocol");
    //         // let conn = factory.create_accepted_connection(cno, stream, remote, BytesMut::new(), buff);
    //         let _handle = tokio::spawn(serve_root());
    //     }
    //     ConnectionProtocol::HttpPcp => {
    //         error!("PeerCastHttp is not allowed");
    //         let _ = stream.shutdown().await;
    //     }
    //     ConnectionProtocol::Http => {
    //         warn!(?cno, ?remote, "STREAM is HTTP Protocol");
    //         // serve_http(cno, stream, remote, graceful_shutdown, force_shutdown).await
    //         let _ = stream.shutdown().await;
    //     }
    //     ConnectionProtocol::Unknown => {
    //         warn!(?cno, ?remote, "STREAM is Unkwon Protocol");
    //         let _ = stream.shutdown().await;
    //     }
    // };
 */
