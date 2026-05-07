// use std::net::SocketAddr;
// use std::sync::Arc;

use anyhow::Context;
use clap::Parser;
use libpeercast_re::{GnuId, connection::shared::connection_factory, repository::shared};
use peercast_root::{
    config::{YpAppendSystemStatus, YpAppendUserStatus},
    connection::RootConnectionSpec,
    service::yellow_page,
};
use tokio_util::sync::CancellationToken;
use tracing::info;

mod app;
use crate::app::{cli, http::server_http, logging, peercast::server_peercast, state::ArcState};

#[cfg(test)]
mod test_helper;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    peercast_root::init();
    let args = cli::Args::parse();
    cli::version_print(&args)?;

    logging::init(&args)?;

    // Init State
    let arc_state: ArcState = {
        use crate::app::state::*;

        let self_session_id = GnuId::new();
        let db_pool = {
            use sqlx::sqlite::SqlitePoolOptions;
            info!("Connecting to database at {}", args.database_url.as_str());
            let pool = SqlitePoolOptions::new().connect(args.database_url.as_str()).await?;
            pool
        };
        let (ch_factory, ch_repository) = shared::channel_factory(self_session_id.clone().into());

        let embed_tmpl_ctx = EmbedTemplateCtx {
            EMBED_TITLE: args.embed_title.clone(),
            EMBED_YP_NAME: args.embed_yp_name.clone(),
            EMBED_URL_HTTP: args.embed_url_http.clone(),
            EMBED_URL_PCP: args.embed_url_pcp.clone(),
            //
            tracker_yp_name: args.yp_name.clone(),
        };

        let yp_config = yellow_page::SiteConfig {
            yp_name: args.yp_name.clone(),
            listener_hideable: args.yp_listerer_hideable,
            restrict_speed: args.yp_limit_speed,
            max_restrict_level: args.yp_restrict_port_level,
        };
        let yellow_page = yellow_page::YellowPageService::new(yp_config.clone());
        let yellow_page = match args.yp_append_user_status {
            YpAppendUserStatus::None => yellow_page,
            YpAppendUserStatus::Default => {
                yellow_page.add_create_user_status_func(yellow_page::create_user_status_default_function(&yp_config))
            }
        };
        let yellow_page = match args.yp_append_system_status {
            YpAppendSystemStatus::None => yellow_page,
            YpAppendSystemStatus::Default => {
                yellow_page.add_create_sys_status_func(yellow_page::create_system_status_default_function(&yp_config))
            }
            YpAppendSystemStatus::WithHost => yellow_page
                .add_create_sys_status_func(yellow_page::create_system_status_with_host_info_function(&yp_config)),
        };

        args.create_dummy_channel.then(|| {
            use libpeercast_re::model::{ValidChannelInfo, ValidTrackInfo};
            use libpeercast_re::repository::traits::ChannelFactory;
            use peercast_root::repository::RootChannelConfig;

            fn dummy_channel(i: usize, genre: &str) -> (GnuId, ValidChannelInfo, ValidTrackInfo, RootChannelConfig) {
                let cid = GnuId::from(0x123456789ABCDEF_u128 + i as u128);
                let channel_info = libpeercast_re::model::ValidChannelInfo {
                    name: format!("名前({})", genre),
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
                let dummy_config = RootChannelConfig {
                    broadcast_id: Some(GnuId::from(0xFEDCBA987654321_u128)),
                    tracker_addr: Some("127.0.0.1:7144".parse().unwrap()),
                };

                (cid, channel_info, track_info, dummy_config)
            }

            let vars = vec![
                dummy_channel(0, "dummy"),
                dummy_channel(1, "ypdummy"),
                dummy_channel(2, "yp@dummy"),
                dummy_channel(3, "yp@@dummy"),
                dummy_channel(4, "yp@@@dummy"),
            ];

            for v in vars {
                // info!("{:?}", v);
                ch_factory.create_or_get(v.0, v.1, v.2, v.3);
            }
        });

        let (conn_factory, conn_manager) = connection_factory::<RootConnectionSpec>(self_session_id.clone().into());

        AppState {
            self_session_id,
            api_config: ApiConfig {
                allow_cors: args.allow_cors.iter().filter(|s| s.is_empty()).map(|s| s.to_string()).collect(),
                use_outer_html_dir: args.use_outer_html_dir.clone(),
                cache_max_age: args.cache_max_age,
                client_ip_source: args.client_ip_source.clone(),
                embed_tmpl_ctx,
            },
            db_pool,
            //
            channel: ChannelRepo {
                repository: ch_repository,
                factory: ch_factory,
            },
            yellow_page,
            //
            conns: ConnectionRepo {
                manager: conn_manager,
                factory: conn_factory,
            },
        }
        .into()
    };
    let cancell_token = CancellationToken::new();

    // Init Socket
    let listener_pcp = tokio::net::TcpListener::bind((args.bind, args.port)).await?;
    info!("PCP listening on pcp://{}", listener_pcp.local_addr().unwrap(),);
    let listener_http = tokio::net::TcpListener::bind((args.api_bind, args.api_port)).await?;
    info!("HTTP listening on http://{}", listener_http.local_addr().unwrap(),);

    // Run Servers
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

    // Wait for shutdown signal
    while let Some(res) = set.join_next().await {
        let r = res.context("A server thread has panicked")?;
        info!("A server thread has shut down : {:?}", r);
    }

    Ok(())
}
