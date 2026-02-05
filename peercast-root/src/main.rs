use std::{
    net::SocketAddr,
    process::exit,
    sync::Arc,
    time::Duration,
};

use anyhow::Context;
use axum::serve::Listener;
use axum_extra::headers::Header;
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use clap::Parser;
use futures_util::{FutureExt, StreamExt};
use libpeercast_re::pcp::{
        ChannelInfo, GnuId, PcpConnectionFactory,
    };
use peercast_root::{
    ExitCode, FooterToml, IndexInfo,
    channel::RootConfig,
    repository::ChannelRepository,
};
// use peercast_re_api::models::channel_info;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

// App modules
mod app;
use app::cli;
use app::logging;

use crate::app::{ApiConfig, AppState, ArcState, config::create_config, server_http, server_peercast};

#[cfg(test)]
mod test_helper;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = cli::Args::parse();
    cli::version_print(&args)?;

    logging::init(&args)?;
    let _config = create_config(&args)?;
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
        server_http::serve(args.clone(), arc_state.clone(), listener_http, cancell_token.child_token()),
    )?;
    set.build_task().name("RootServer").spawn(
        // server_peercast(shutdown_token.child_token(), store.clone(), svr_listener),
        server_peercast::serve(args.clone(), arc_state, listener_pcp, cancell_token.child_token()),
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
        let t = FooterToml::from_path(path)
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
