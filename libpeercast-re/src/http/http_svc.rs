use std::{
    collections::HashMap, convert::Infallible, future, net::SocketAddr, path::PathBuf,
    str::FromStr, sync::Arc, task::Poll, time::Duration,
};

use askama::filters::format;
use axum::{
    body::{self, Body},
    extract::{connect_info::Connected, ConnectInfo, Path, Query, Request, State},
    http::HeaderValue,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{self, get},
    Router,
};
use axum_core::BoxError;
use axum_extra::extract::Host;
use bytes::Bytes;
use futures_util::{future::Pending, task::SpawnExt, Stream};
use hyper::{rt::Write, upgrade::Upgraded, StatusCode, Uri};
use hyper_util::rt::TokioIo;
use rml_rtmp::sessions::StreamMetadata;
use rust_embed::RustEmbed;
use serde_json::json;
use tokio::{
    io::AsyncWriteExt,
    net::TcpStream,
    sync::mpsc::{self, unbounded_channel},
};
use tokio_stream::StreamExt;
use tower_http::{
    cors::{self, CorsLayer},
    trace::TraceLayer,
};

use tracing::{debug, error, info, trace, Span};

use crate::{
    codec::FlvWriter,
    config::Config,
    http::{middleware::RestrictIpLayer },
    pcp::{
        ChannelInfo, ChannelManager, ChannelMessage, ChannelType, GnuId, RelayTaskConfig,
        SourceTaskConfig, TaskStatus,
    },
    rtmp::{connection::Connection, stream_manager::StreamManagerMessage},
    ConnectionId,
};

#[cfg(debug_assertions)]
use super::UiProxyMode;

use super::{Api, AppState, MyConnectInfo};

const VITE_UI_PORT: u16 = 5173;
const SWAGGER_UI_PORT: u16 = 8002;
const SWAGGER_EDITOR_PORT: u16 = 8001;

pub struct HttpSvc;
impl HttpSvc {
    pub fn new(
        config_path: PathBuf,
        config: Config,
        session_id: GnuId,
        channel_manager: Arc<ChannelManager>,
        manager_sender: Arc<mpsc::UnboundedSender<StreamManagerMessage>>,
    ) -> Router<()> {
        #[cfg(debug_assertions)]
        let proxy_mode = match std::env::var("PEERCAST_RT_FRONTEND_UI_MODE") {
            Ok(val) => {
                if val.to_uppercase() == "PROXY" {
                    UiProxyMode::Proxy
                } else {
                    UiProxyMode::Embed
                }
            }
            Err(_) => UiProxyMode::Embed,
        };

        let port = config.server_port;
        let mut origins = vec![format!("http://localhost:{port}")
            .parse::<HeaderValue>()
            .unwrap()];
        if cfg!(debug_assertions) {
            origins.push(
                format!("http://localhost:{VITE_UI_PORT}")
                    .parse::<HeaderValue>()
                    .unwrap(),
            );
            origins.push(
                format!("http://localhost:{SWAGGER_UI_PORT}")
                    .parse::<HeaderValue>()
                    .unwrap(),
            );
            origins.push(
                format!("http://localhost:{SWAGGER_EDITOR_PORT}")
                    .parse::<HeaderValue>()
                    .unwrap(),
            );
        }
        let headers = [hyper::header::CONTENT_TYPE];

        // local_address
        let allow_ips = config.local_address.clone();

        debug!(cor_origins=?origins);
        debug!(cor_headers=?headers);
        debug!(allow_ips=?allow_ips);

        Router::new()
            .route("/", get(Self::handler))
            .route("/pls/:id", get(Self::playlist))
            .route("/stream/:id", get(Self::stream))
            // .route("/demo/throttle", get(Demo::throttle))
            // .route("/ui", get(|| async { Redirect::permanent("/ui/") }))
            // .nest("/ui/", Ui::new())
            .nest("/api", Api::new())
            .fallback(Self::not_found)
            .layer(TraceLayer::new_for_http().on_body_chunk(
                |chunk: &Bytes, _latency: Duration, _span: &Span| {
                    tracing::debug!("streaming {} bytes", chunk.len());
                },
            ))
            .layer(RestrictIpLayer {
                white_nets: allow_ips,
            })
            .layer(
                CorsLayer::new()
                    .allow_origin(origins)
                    .allow_methods(cors::Any)
                    .allow_headers(headers),
            )
            .with_state(AppState {
                channel_manager,
                manager_sender,
                //
                config_path,
                config,
                session_id,
                //
                #[cfg(debug_assertions)]
                proxy_mode,
            })
    }

    async fn not_found(req: Request) -> Html<&'static str> {
        debug!("not_found(req={req:?})");
        Html("<h1>404</h1><p>Not Found</p>")
    }

    async fn handler(ConnectInfo(info): ConnectInfo<MyConnectInfo>) -> Html<&'static str> {
        info!(?info);
        Html("<h1>Hello, World!</h1>")
    }

    // async fn playlist(req: Request) -> impl IntoResponse {
    async fn playlist(
        ConnectInfo(MyConnectInfo {
            // local,
            remote,
            connection_id,
            shutdown,
        }): ConnectInfo<MyConnectInfo>,
        Host(host): Host,
        Path(channel_id): Path<String>,
        Query(params): Query<HashMap<String, String>>,
        State(AppState {
            channel_manager,
            session_id,
            config,
            ..
        }): State<AppState>,
    ) -> impl IntoResponse {
        // debug!(?connection_id, ?host, ?channel_id, ?params);
        let split_chid = channel_id.split(".").collect::<Vec<&str>>();
        let Some((channel_id, extentions)) = split_chid.split_first() else {
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        };

        let Ok(channel_id) = GnuId::from_str(channel_id) else {
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        };

        let ch = match channel_manager.get(&channel_id) {
            Some(ch) => ch,
            None => {
                let Ok(connect_to) = params["tip"].parse::<SocketAddr>() else {
                    error!("tipに接続先のIpアドレスが含まれていません");
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                };
                let ch_type = ChannelType::Relay;
                channel_manager.create_or_get(channel_id, ch_type, None, None)
            }
        };

        // TODO: Channelから情報もってきて適宜処理する
        match ch.status() {
            TaskStatus::Receiving => { /* pass */ }
            // FIXME: session_idをどこかで管理すること
            _ => {
                // let _ = ch.connect(ConnectionId::new(), session_id, connect_to);
                let connect_to = "192.168.10.230:61744".parse().unwrap();
                let task_config = SourceTaskConfig::Relay(RelayTaskConfig {
                    addr: connect_to,
                    self_addr: todo!(),
                });
                let _ = ch.connect(connection_id, task_config);
            }
        };
        drop(ch);

        // TODO: chから拡張子をゲットする
        // let _extensions = extentions
        //     .first()
        //     .map_or_else(|| String::new(), |ext| format!(".{}", ext));
        let (host, port) = match host.parse::<Uri>() {
            Ok(host_url) => {
                //
                let host = host_url.host().unwrap_or_else(|| "localhost").to_string();
                let port = host_url
                    .port()
                    .map(|port| port.as_u16())
                    .unwrap_or_else(|| config.server_port);
                (host, port)
            }
            Err(e) => {
                error!("Invalid Uri {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        };

        // < Content-Type:audio/x-mpegurl
        // #EXTM3U
        // #EXTINF:-1, [CHANNEL_NAME]
        // http://[ADDR]:[PORT]/stream/[GnuID].flv
        let m3u_str = indoc::formatdoc! {"
            #EXTM3U
            #EXTINF:-1, [CHANNEL_NAME]
            http://{host}:{port}/stream/{channel_id}
        "};

        // trace!(?connection_id, ?m3u_str);

        Ok((
            StatusCode::OK,
            [(hyper::header::CONTENT_TYPE, "audio/x-mpegurl")],
            m3u_str,
        ))
    }

    // http://192.168.1.10:17144/stream/85B32473FE39A93B60276926BB966CEA.flv
    async fn stream(
        ConnectInfo(conn): ConnectInfo<MyConnectInfo>,
        Path(channel_id): Path<String>,
        State(state): State<AppState>,
    ) -> impl IntoResponse {
        trace!(?channel_id);
        let Ok(channel_id) = GnuId::from_str(&channel_id) else {
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        };
        let Some(channel) = state.channel_manager.get(&channel_id) else {
            return Err(StatusCode::NOT_FOUND);
        };

        let mut streamer = channel.channel_stream(conn.connection_id);
        trace!("streamer={:?}", &streamer);
        drop(channel);

        let resp = Response::builder()
            .status(StatusCode::OK)
            .header(hyper::header::CONTENT_TYPE, "video/x-flv")
            .body(Body::from_stream(streamer))
            .unwrap();

        Ok(resp)
    }
}

// AsyncWriteをStreamにする場合
// let mut reciever = channel.channel_reciever(conn.connection_id);
// trace!("reciever={:#?}", &reciever);
// let (writer, reader) = tokio::io::duplex(1024 * 1024);
// let (_, writer) = tokio::io::split(writer);
// let (reader, _) = tokio::io::split(reader);
// tokio::spawn(async move {
//     let mut flv = FlvWriter::new(writer);
//     // let mut file = tokio::fs::File::create("./tmp/stream.flv.txt")
//     //     .await
//     //     .unwrap();
//     let mut is_sent_magic = false;
//     loop {
//         let Some(msg) = reciever.recv().await else {
//             break;
//         };
//         match msg {
//             ChannelMessage::AtomChanHead { atom, pos, data } => {
//                 //
//                 is_sent_magic = true
//                 // stream.write(data)
//             }
//             ChannelMessage::AtomChanData { pos, data } => todo!(),
//         }
//     }
//     info!("FINISHE FLVWrite Thread {}", conn.connection_id);
// });
// let streamer = tokio_util::io::ReaderStream::new(reader);

// struct Demo;
// impl Demo {
//     async fn throttle() -> impl IntoResponse {
//         "aiueo"
//     }
// }

// async fn stream_some_data() -> Body {
//     let stream = tokio_stream::iter(0..5)
//         .throttle(Duration::from_secs(1))
//         .map(|n| format!("{n}\n"))
//         .map(Ok::<_, Infallible>);
//     Body::from_stream(stream)
// }

// async fn tunnel(mut upgraded: Upgraded, addr: String) -> std::io::Result<()> {
//     use tokio::io::AsyncReadExt;
//     let mut server = TcpStream::connect(addr).await?;
//     let io = TokioIo::new(upgraded);

//     Ok(())
// }

#[cfg(test)]
mod test_spec {

    use bytes::BytesMut;
    use tokio::io::*;

    #[crate::test]
    async fn test_io() {
        // duplex使えば簡単に実装できそう
        let (mut writer, mut reader) = duplex(1024 * 1024);

        let _ = writer.write_i8(1).await.unwrap();
        let v = reader.read_i8().await.unwrap();
        assert_eq!(v, 1);

        let _ = writer.write_i8(-10).await.unwrap();
        let v = reader.read_i8().await.unwrap();
        assert_eq!(v, -10);
    }
}
