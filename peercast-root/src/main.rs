use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Context;
use clap::Parser;
use libpeercast_re::{
    model::{ValidChannelInfo, ValidTrackInfo},
    pcp::{GnuId, connection5::shared},
    prelude::*,
    repository::Repository,
};
use peercast_root::{
    YpAppendSystemStatus, YpAppendUserStatus,
    channel::RootConfig,
    config::FooterToml,
    connection::RootSpec,
    model::IndexInfo,
    repository::RootRepository2,
    service::{
        SiteConfig, YellowPageService, createSystemStatusDefaultFunction, createSystemStatusWithHostInfoFunction,
        createUserStatusDefaultFunction,
    },
};

use tokio_util::sync::CancellationToken;

// App modules
mod app;
use app::cli;
use app::logging;

use crate::app::{ApiConfig, AppState, ArcState, server_http, server_peercast};

#[cfg(test)]
mod test_helper;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = cli::Args::parse();
    cli::version_print(&args)?;

    logging::init(&args)?;
    let cancell_token = CancellationToken::new();
    let arc_state = init(&args, GnuId::new(), (args.bind, args.port).into()).await?;

    // Init Socket
    let listener_pcp = tokio::net::TcpListener::bind((args.bind, args.port)).await?;
    info!("PCP listening on pcp://{}", listener_pcp.local_addr().unwrap(),);
    let listener_http = tokio::net::TcpListener::bind((args.api_bind, args.api_port)).await?;
    info!("HTTP listening on http://{}", listener_http.local_addr().unwrap(),);

    let mut set = tokio::task::JoinSet::new();
    set.build_task().name("ApiServer").spawn(server_http(
        arc_state.clone(),
        listener_http,
        cancell_token.child_token(),
    ))?;
    set.build_task().name("RootServer").spawn(server_peercast(arc_state, listener_pcp, cancell_token.child_token()))?;
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

async fn init(args: &cli::Args, self_session_id: GnuId, _self_socket: SocketAddr) -> anyhow::Result<ArcState> {
    peercast_root::init();
    let (connection_factory, connection_manager) = shared::connection_factory::<RootSpec>();

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

    let repository = RootRepository2::new(|_| async {}).await;

    if args.create_dummy_channel {
        create_dummy_channel(&repository).await;
    }

    let api_config = ApiConfig {
        allow_cors: args.allow_cors.iter().filter(|s| s.is_empty()).map(|s| s.to_string()).collect(),
        cache_max_age: args.cache_max_age,
        client_ip_source: args.client_ip_source.clone(),
    };

    let db_pool = init_db(args).await?;

    // YellowPageの設定
    let yp_config = SiteConfig {
        yp_name: args.yp_name.clone(),
        listener_hideable: args.yp_listerer_hideable,
        restrict_speed: args.yp_limit_speed,
        max_restrict_level: args.yp_restrict_port_level,
    };
    let yellow_page = YellowPageService::new(yp_config.clone()).add_footer_channels(index_txt_footer.clone());
    let yellow_page = match args.yp_append_user_status {
        YpAppendUserStatus::None => yellow_page,
        YpAppendUserStatus::Default => {
            yellow_page.add_create_user_status_func(createUserStatusDefaultFunction(&yp_config))
        }
    };
    let yellow_page = match args.yp_append_system_status {
        YpAppendSystemStatus::None => yellow_page,
        YpAppendSystemStatus::Default => {
            yellow_page.add_create_sys_status_func(createSystemStatusDefaultFunction(&yp_config))
        }
        YpAppendSystemStatus::WithHost => {
            yellow_page.add_create_sys_status_func(createSystemStatusWithHostInfoFunction(&yp_config))
        }
    };
    let yellow_page = Arc::new(yellow_page);

    // AppStateの作成
    let app_sate = AppState {
        self_session_id,
        config: Arc::new(api_config),
        index_txt_footer,
        db_pool,
        yellow_page,
        repository,
        connection_factory,
        connection_manager,
    };

    Ok(ArcState(Arc::new(app_sate)))
}

async fn init_db(args: &cli::Args) -> anyhow::Result<sqlx::Pool<sqlx::sqlite::Sqlite>> {
    use sqlx::sqlite::SqlitePoolOptions;
    info!("Connecting to database at {}", args.database_url.as_str());
    let pool = SqlitePoolOptions::new().connect(args.database_url.as_str()).await?;
    Ok(pool)
}

async fn create_dummy_channel(repository: &RootRepository2) {
    let vars = vec![
        dummy_channel(0, "dummy"),
        dummy_channel(1, "ypdummy"),
        dummy_channel(2, "yp@dummy"),
        dummy_channel(3, "yp@@dummy"),
        dummy_channel(4, "yp@@@dummy"),
    ];

    for v in vars {
        println!("{:?}", v);
        repository.create_or_get(v.0, Some(v.1), Some(v.2), Some(v.3)).await;
    }
}

fn dummy_channel(i: usize, genre: &str) -> (GnuId, ValidChannelInfo, ValidTrackInfo, RootConfig) {
    let cid = GnuId::from(0x123456789ABCDEF_u128 + i as u128);
    let channel_info = libpeercast_re::model::ValidChannelInfo {
        name: "Dummyチャンネル名".to_string(),
        url: "http://example.com".to_string(),
        genre: genre.to_string(),
        desc: "This is a dummy channel desc".to_string(),
        comment: "No comments.".to_string(),
        stream_type: "video/x-flv".to_string(),
        stream_ext: ".flv".to_string(),
        bitrate: 128,
        typee: "FLV".to_string(),
    };
    let track_info = libpeercast_re::model::ValidTrackInfo {
        title: "Dummy Track".to_string(),
        creator: "Dummy Artist".to_string(),
        url: "http://example.com/track".to_string(),
        album: "Dummy Album".to_string(),
        genre: "Various".to_string(),
    };
    let dummy_config = RootConfig {
        broadcast_id: GnuId::from(0xFEDCBA987654321_u128),
        tracker_addr: Some("127.0.0.1:7144".parse().unwrap()),
    };

    (cid, channel_info, track_info, dummy_config)
}
