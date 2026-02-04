#![allow(unused)]
use std::{
    any,
    net::{IpAddr, Shutdown, SocketAddr},
    path::PathBuf,
    process::exit,
    sync::{Arc, Mutex, OnceLock, RwLock},
    time::{Duration, Instant},
};

use anyhow::Context;
use axum::{
    Json, Router,
    extract::Query,
    http::{HeaderValue, Method},
    response::IntoResponse,
    routing,
    serve::Listener,
};
use axum_extra::headers::Header;
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use bytes::BytesMut;
use chrono::{DateTime, TimeZone, Utc};
use clap::Parser;
use futures_util::{FutureExt, SinkExt, StreamExt, future::BoxFuture};
use itertools::concat;
use libpeercast_re::{
    ConnectionNo, config,
    pcp::{
        ChannelInfo, GnuId, Id4, ParentAtom, PcpConnectionFactory, TrackInfo,
        builder::{QuitBuilder, QuitReason, RootBuilder},
        connection::PcpConnection,
        decode::{PcpBroadcast, PcpChannel, PcpHost},
        procedure::PcpHandshake,
        repository,
    },
    util::{ConnectionProtocol, identify_protocol, mutex_poisoned, rwlock_read_poisoned, rwlock_write_poisoned},
};
use peercast_root::{
    ExitCode, FooterToml, IndexInfo, RestrictPortLevel,
    channel::{RootChannel, RootConfig, get_tracker_addr},
    repository::ChannelRepository,
};
// use peercast_re_api::models::channel_info;
use serde::{Deserialize, Serialize};
use serde_json::value::Index;
use serde_with::{NoneAsEmptyString, serde_as};
use tokio::{
    fs::read,
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
    sync::watch,
    time::Interval,
};
use tokio_util::sync::CancellationToken;
use tower_http::{cors::CorsLayer, services::ServeDir, set_header::SetResponseHeaderLayer};
use tracing::{debug, error, info, instrument::WithSubscriber, trace, warn};
use url::Url;

// App modules
mod app;
use app::cli;
use app::handler;
use app::logging;

use crate::app::{ApiConfig, AppState, ArcState};

#[cfg(test)]
mod test_helper;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = cli::Args::parse();
    cli::version_print(&args)?;

    logging::init(&args)?;
    let arc_state = init_app(&args, GnuId::new(), (args.bind, args.port).into()).await;

    // Init socket
    let listener_pcp = tokio::net::TcpListener::bind((args.bind, args.port)).await?;
    info!("PCP listening on pcp://{}", listener_pcp.local_addr().unwrap(),);

    let listener_http: TcpListener = tokio::net::TcpListener::bind((args.api_bind, args.api_port)).await?;
    info!("HTTP listening on http://{}", listener_http.local_addr().unwrap(),);

    let cancell_token = CancellationToken::new();
    let mut set = tokio::task::JoinSet::new();
    set.build_task().name("ApiServer").spawn(
        //
        server_api(args.clone(), arc_state.clone(), listener_http, cancell_token.child_token()),
    )?;
    set.build_task().name("RootServer").spawn(
        // server_peercast(shutdown_token.child_token(), store.clone(), svr_listener),
        server_peercast(args.clone(), arc_state, listener_pcp, cancell_token.child_token()),
    )?;
    set.build_task().name("WaitShutdownSig").spawn(async move {
        tokio::signal::ctrl_c().await.expect("failed to listen for event");
        cancell_token.cancel();
        info!("SHUTDOWN SIGNAL SENT");
        anyhow::Ok(())
    })?;

    while let Some(res) = set.join_next().await {
        let r = res.context("A server thread has panicked")?;
        info!("A server thread has shut down : {:?}", r);
    }

    Ok(())
}

async fn init_app(args: &cli::Args, self_session_id: GnuId, self_socket: SocketAddr) -> ArcState {
    // _REPOSITORY.get_or_init(|| ChannelRepository::new(&self_session_id));
    let conn_factory = PcpConnectionFactory::new(self_session_id, self_socket);

    let mut index_txt_footer = vec![];
    if let Some(ref path) = args.index_txt_footer {
        let mut t = FooterToml::from_path(path)
            .with_context(|| {
                let p = path.display();
                format!("index.txtのフッターファイル({p})の読み込みに失敗しました。")
            })
            .unwrap();
        let mut infos: Vec<IndexInfo> = t.infomations.into_iter().map(|i| i.into()).collect();
        dbg!(&infos);
        index_txt_footer.append(&mut infos);
    }

    let repository = ChannelRepository::new(&self_session_id);
    if args.create_dummy_channel {
        let mut chinfo = ChannelInfo::new();
        let level_fmt = match args.yp_restrict_port_level {
            peercast_root::RestrictPortLevel::None => "",
            peercast_root::RestrictPortLevel::PortCheck => "@",
            peercast_root::RestrictPortLevel::BroadcastSpeed => "@@",
            peercast_root::RestrictPortLevel::RestrictSpeed => "@@@",
            v => {
                error!("Invalid port check level: {:?}", v);
                exit(ExitCode::Failure as i32);
            }
        };
        chinfo.name = "ダミーチャンネル".into();
        chinfo.genre = format!("{}{}ダミージャンル", args.yp_name_space, level_fmt).into();
        chinfo.comment = "ダミーチャンネルはおおよそ5分後に消えます".into();
        chinfo.url = "https://yp-dev.007144.xyz/".into();
        chinfo.typ = "RAW".into();
        let config = RootConfig {
            tracker_host: Some("127.0.0.1:7144".parse().unwrap()),
        };
        repository.create_or_get(GnuId::new(), Some(chinfo), None, Some(config));
    }

    let api_config = ApiConfig {
        restrict_speed: args.yp_limit_speed,
        listener_hideable: args.yp_listerer_hideable,
        port_check_level: args.yp_restrict_port_level,
        name_space: args.yp_name_space.clone(),
    };

    let db_pool = init_db(args).await;

    let app_sate = AppState {
        config: Arc::new(api_config),
        index_txt_footer,
        redis_master_key: args.redis_master_key.clone(),
        db_pool: db_pool,
        repository,
        conn_factory,
    };

    ArcState(Arc::new(app_sate))
}

async fn init_db(args: &cli::Args) -> Pool<RedisConnectionManager> {
    use redis::AsyncCommands;
    use tokio::time::timeout;

    debug!("connecting to redis: {}", args.redis_url);
    let manager = RedisConnectionManager::new(args.redis_url.clone()).unwrap();
    let pool = bb8::Pool::builder().build(manager).await.unwrap();
    {
        // let mut conn = pool.get().await.unwrap();
        let mut conn = match tokio::time::timeout(Duration::from_millis(2000), pool.get()).await {
            Ok(Ok(conn)) => conn,
            Ok(Err(e)) => {
                error!("redis connect failed :{}", e);
                std::process::exit(ExitCode::Failure as i32);
            }
            Err(e) => {
                error!("redis connect timeout: {}", e);
                std::process::exit(ExitCode::Failure as i32);
            }
        };

        let key = format!("{}:CHECK", args.redis_master_key);
        // conn.set::<&str, &str, ()>(&key, "CHECK_ME").await;
        match timeout(Duration::from_millis(2000), conn.set::<&str, &str, ()>(&key, "CHECK_ME")).await {
            Ok(Ok(())) => {
                debug!("redis connect SET COMMAND success");
            }
            Ok(Err(e)) => {
                error!("redis connect SET COMMAND failed: {}", e);
                std::process::exit(ExitCode::Failure as i32);
            }
            Err(_) => {
                error!("redis connect SET COMMAND timeout");
                std::process::exit(ExitCode::Failure as i32);
            }
        };

        match timeout(Duration::from_millis(1000), conn.get::<_, String>(&key)).await {
            Ok(Ok(r)) => {
                debug!("redis connect GET COMMAND success: {}", r);
                assert_eq!(r, "CHECK_ME");
            }
            Ok(Err(e)) => {
                error!("redis connect GET COMMAND failed: {}", e);
                std::process::exit(ExitCode::Failure as i32);
            }
            Err(_) => {
                error!("redis connect GET COMMAND timeout");
                std::process::exit(ExitCode::Failure as i32);
            }
        };
    }
    tracing::debug!("successfully connected to redis and pinged it");

    pool
}

async fn server_api(
    args: cli::Args,
    state: ArcState,
    listener: TcpListener,
    graceful_shutdown: CancellationToken,
) -> anyhow::Result<()> {
    use bb8_redis::RedisConnectionManager;
    use tower_http::trace::{DefaultMakeSpan, TraceLayer};

    let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    info!("asset_dir: {:?}", &assets_dir);

    let cor_origins: Vec<_> = args.allow_cors.iter().map(|origin| origin.parse::<HeaderValue>().unwrap()).collect();
    info!("cor_origins: {:?}", &cor_origins);

    let cache_control_value = format!("max-age={}, public, mustrelvalidate", &args.cache_max_age);
    info!("cache-control: {}", &cache_control_value);

    let tracker = tokio_util::task::TaskTracker::new();
    info!("START HTTP SERVER");

    let app = Router::new()
        .fallback_service(ServeDir::new(assets_dir).append_index_html_on_directories(true))
        .route("/index.txt", routing::get(handler::index_txt))
        .route("/api/index.json", routing::get(handler::index_json))
        .layer(TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::default().include_headers(true)))
        .layer(CorsLayer::new().allow_origin(cor_origins).allow_methods([Method::GET]))
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::CACHE_CONTROL,
            HeaderValue::from_bytes(cache_control_value.as_bytes()).unwrap(),
        ))
        .layer(args.ip_source.into_extension())
        .with_state(state);

    let _ = axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(graceful_shutdown.cancelled_owned())
        .await;

    Ok(())
}

async fn server_peercast(
    args: cli::Args,
    state: ArcState,
    listener: TcpListener,
    graceful_shutdown: CancellationToken,
) -> anyhow::Result<()> {
    // スレッドの終了を検知するためのチャンネル
    let (closed_tx, closed_rx) = tokio::sync::watch::channel(());
    let tracker = tokio_util::task::TaskTracker::new();
    info!("START PCP SERVER");

    'accept: loop {
        println!("loop start");
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
                        // let _handle = spawner.spawn(serve_peercast( cid, stream, addr, child_graceful_shutdown, closed_rx));
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

#[inline]
async fn serve_peercast(
    state: ArcState,
    cid: ConnectionNo,
    mut stream: TcpStream,
    remote: SocketAddr,
    graceful_shutdown: CancellationToken,
    closed_send: watch::Receiver<()>,
) {
    info!(?cid, ?remote, "SPAWN SERVE");
    match identify_protocol(&stream).await {
        Ok(ConnectionProtocol::PeerCast) => {
            serve_root(state, cid, stream, remote, graceful_shutdown, closed_send).await
        }
        Ok(ConnectionProtocol::PeerCastHttp) => {
            error!("PeerCastHttp is not allowed");
            let _ = stream.shutdown().await;
        }
        Ok(ConnectionProtocol::Http) => {
            warn!(?cid, ?remote, "STREAM is HTTP Protocol");
            // serve_http(cid, stream, remote, graceful_shutdown, force_shutdown).await
            let _ = stream.shutdown().await;
        }
        Ok(ConnectionProtocol::Unknown) => {
            warn!(?cid, ?remote, "STREAM is Unkwon Protocol");
            let _ = stream.shutdown().await;
        }
        Err(e) => {
            error!(?cid, ?remote, "Failed: identify_protocol: {}", e);
            let _ = stream.shutdown().await;
        }
    }
}

//-------------------------------------------------------------------------------
// PCP
//-------------------------------------------------------------------------------

async fn serve_root(
    state: ArcState,
    cid: ConnectionNo,
    mut stream: TcpStream,
    remote: SocketAddr,
    graceful_shutdown: CancellationToken,
    closed_send: watch::Receiver<()>,
) {
    use libpeercast_re::pcp::connection::HandshakeType;

    let read_buf = BytesMut::new();

    // HandshakeFutureにすればよさそう
    let handshake = state.0.conn_factory.accept(cid, stream, remote);

    // Handshake時に送ってもらうAtomを作成する
    let root_atom = RootBuilder::default().set_update_interval(10).set_next_update_interval(10).build();

    let mut conn = match handshake.incoming(root_atom.into()).await {
        Err(e) => {
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

    let (channel_id_in_chpkt, braodcast_id) = match (channel_id, broadcast_id) {
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
        tracker_host,
    };

    // 対象チャンネルを取得
    let repo = &state.0.repository;
    let ch = repo.create_or_get(*channel_id_in_bcst, channel_info, track_info, Some(config));

    // Channelにコネクションを接続
    let attach_task = ch.attach_connection(conn, graceful_shutdown, closed_send);
    attach_task.await;
}
