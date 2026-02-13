use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Context;
use clap::Parser;
use libpeercast_re::pcp::GnuId;
use libpeercast_re::pcp::connection2::shared_connection_factory;
use libpeercast_re::repository::Repository;
use peercast_root::prelude::*;
use peercast_root::repository::RootRepository2;
use peercast_root::{FooterToml, IndexInfo};
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
    let arc_state = init(&args, GnuId::new(), (args.bind, args.port).into()).await?;

    // Init Socket
    let listener_pcp = tokio::net::TcpListener::bind((args.bind, args.port)).await?;
    info!("PCP listening on pcp://{}", listener_pcp.local_addr().unwrap(),);
    let listener_http = tokio::net::TcpListener::bind((args.api_bind, args.api_port)).await?;
    info!("HTTP listening on http://{}", listener_http.local_addr().unwrap(),);

    let cancell_token = CancellationToken::new();
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

async fn init(args: &cli::Args, _self_session_id: GnuId, _self_socket: SocketAddr) -> anyhow::Result<ArcState> {
    // _REPOSITORY.get_or_init(|| ChannelRepository::new(&self_session_id));
    let (connection_factory, connection_manager) = shared_connection_factory();

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

    let repository2 = RootRepository2::new(|_| async {}).await;

    if args.create_dummy_channel {
        let level_fmt = match args.yp_restrict_port_level {
            peercast_root::RestrictPortLevel::None => "",
            peercast_root::RestrictPortLevel::PortCheck => "@",
            peercast_root::RestrictPortLevel::BroadcastSpeed => "@@",
            peercast_root::RestrictPortLevel::RestrictSpeed => "@@@",
        };

        let dummy_channel_id = GnuId::from(0x123456789ABCDEF_u128);
        let dummy_channel_info = libpeercast_re::pcp::ChannelInfo {
            name: "Dummyチャンネル名".to_string(),
            url: "http://example.com".to_string(),
            genre: format!("{}{}ダミージャンル", args.yp_name_space, level_fmt).into(),
            desc: "This is a dummy channel desc".to_string(),
            comment: "No comments.".to_string(),
            stream_type: "video/x-flv".to_string(),
            stream_ext: ".flv".to_string(),
            bitrate: 128,
            typ: "FLV".to_string(),
        };
        let dummy_track_info = libpeercast_re::pcp::TrackInfo {
            title: "Dummy Track".to_string(),
            creator: "Dummy Artist".to_string(),
            url: "http://example.com/track".to_string(),
            album: "Dummy Album".to_string(),
            genre: "Various".to_string(),
        };

        repository2.create_or_get(dummy_channel_id, Some(dummy_channel_info), Some(dummy_track_info), None).await;
    }

    let api_config = ApiConfig {
        restrict_speed: args.yp_limit_speed,
        listener_hideable: args.yp_listerer_hideable,
        port_check_level: args.yp_restrict_port_level,
        name_space: args.yp_name_space.clone(),
        allow_cors: args.allow_cors.clone(),
        cache_max_age: args.cache_max_age,
        client_ip_source: args.ip_source.clone(),
    };

    let db_pool = init_db(args).await?;

    let yellow_page = app::yp::YellowPage::new(&args.yp_name_space).add_footer_channels(index_txt_footer.clone());
    let yellow_page = Arc::new(yellow_page);

    let app_sate = AppState {
        config: Arc::new(api_config),
        index_txt_footer,
        db_pool,
        yellow_page,
        repository2,
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
