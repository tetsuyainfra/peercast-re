use std::{net::{SocketAddr, }, path::PathBuf};

use axum::{extract::Query, http::HeaderValue, response::IntoResponse, routing, Json, Router};
use chrono::{DateTime, TimeZone, Utc};
use futures_util::{future::BoxFuture, FutureExt};
use hyper::Method;
use libpeercast_re::pcp::{ChannelInfo, GnuId, TrackInfo};
use peercast_root::IndexInfo;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, NoneAsEmptyString};
use tokio_util::sync::CancellationToken;
use tower_http::{cors::CorsLayer, set_header::SetResponseHeaderLayer};
use tracing::{info, warn};

use crate::{cli, RootChannel, INDEX_TXT_FOOTER, REPOSITORY};


//-------------------------------------------------------------------------------
// HTTP
//-------------------------------------------------------------------------------
pub async fn server_http(
    args: cli::Args,
    listener: tokio::net::TcpListener,
    graceful_shutdown: CancellationToken,
) -> anyhow::Result<()> {
    use axum::routing::any;
    use tower_http::{
        services::ServeDir,
        trace::{DefaultMakeSpan, TraceLayer},
    };

    let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    info!("asset_dir: {:?}", &assets_dir);

    let cor_origins: Vec<_> = args
        .allow_cors
        .iter()
        .map(|origin| origin.parse::<HeaderValue>().unwrap())
        .collect();
    info!("cor_origins: {:?}", &cor_origins);

    let cache_control_value = format!("max-age={}, public, mustrelvalidate", &args.cache_max_age);
    info!("cache-control: {}", &cache_control_value);

    let tracker = tokio_util::task::TaskTracker::new();
    info!("START HTTP SERVER");

    let app = Router::new()
        .fallback_service(ServeDir::new(assets_dir).append_index_html_on_directories(true))
        .route("/index.txt", routing::get(index_txt))
        .route("/api/index.json", routing::get(index_json))
        // logging so we can see what's going on
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(cor_origins)
                // .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap())
                .allow_methods([Method::GET]),
        )
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::CACHE_CONTROL,
            HeaderValue::from_bytes(cache_control_value.as_bytes()).unwrap()
        ));

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal(graceful_shutdown ))
    .await
    .unwrap();

    Ok(())
}

fn shutdown_signal(
    graceful_shutdown: CancellationToken,
) -> BoxFuture<'static, ()> {
    async move {
        //
        graceful_shutdown.cancelled().await;
        info!("HTTP start graceful shutdown");
    }
    .boxed()
}

async fn index_txt(
    Query(params): Query<IndexTextParams>
) -> impl IntoResponse {
    if let Some(host )= params.host {
        warn!(?host, "NOT IMPLEMENTED {}:{}", file!(), line!());
    }

    let channels: Vec<String> = merged_channels()
        .iter()
        .map(|c| c.to_line_of_index_txt())
        .collect();

    itertools::join(channels, "\n")
}

async fn index_json() -> Json<Vec<JsonChannel>> {
    merged_channels().into()
}

fn merged_channels() -> Vec<JsonChannel> {
    let mut channels: Vec<JsonChannel> = REPOSITORY().map_collect(|(id, ch)| ch.into());

    channels.reserve(INDEX_TXT_FOOTER().len());
    channels.extend(INDEX_TXT_FOOTER().clone().into_iter().map(|e| e.into()));
    channels
}


/// index.txtに対するクエリ型
#[serde_as]
#[derive(Debug, Deserialize)]
struct IndexTextParams {
    #[serde_as(as = "NoneAsEmptyString")]
    pub host: Option<String>,
}


#[derive(Debug, Clone, Serialize)]
pub struct JsonChannel {
    id: GnuId,
    name: String,
    tracker_addr: Option<SocketAddr>,
    contact_url: String,
    genre: String,
    desc: String,
    comment: String,
    stream_type: String,
    stream_ext: String,
    bitrate: i32,
    // filetype: String,
    // status: String,
    number_of_listener: i32,
    number_of_relay: i32,
    created_at: DateTime<Utc>, // FIX: 外部のCDNなどとの兼ね合いで配信時間が00:00意外になる可能性あり
    track: JsonTrack,

    #[serde(rename = "type")]
    typee: String,
}

#[derive(Debug, Clone, Serialize)]
struct JsonTrack {
    title: String,
    creator: String,
    url: String,
    album: String,
    genre: String,
}

impl From<&RootChannel> for JsonChannel {
    fn from(ch: &RootChannel) -> Self {
        let ChannelInfo {
            typ,
            name,
            genre,
            desc,
            comment,
            url,
            stream_type,
            stream_ext,
            bitrate,
        } = ch.channel_info();

        JsonChannel {
            id: ch.id(),
            name,
            tracker_addr: ch.tracker_addr(),
            contact_url: url,
            genre,
            desc,
            comment,
            typee: typ,
            stream_type,
            stream_ext,
            bitrate,
            number_of_listener: ch.number_of_listener(),
            number_of_relay: ch.number_of_relay(),
            created_at: ch.created_at(),
            track: ch.track_info().into(),
        }
    }
}

impl From<TrackInfo> for JsonTrack {
    fn from(t: TrackInfo) -> Self {
        JsonTrack {
            title: t.title,
            creator: t.creator,
            url: t.url,
            album: t.album,
            genre: t.genre,
        }
    }
}

impl JsonChannel {
    fn to_line_of_index_txt(&self) -> String {
        create_index_line(
            &self.name,
            &self.id,
            &self.tracker_addr,
            &self.contact_url,
            &self.genre,
            &self.desc,
            &self.comment,
            self.number_of_listener,
            self.number_of_relay,
            self.bitrate,
            &self.typee,
            &self.stream_type,
            &self.stream_ext,
            &self.created_at,
        )
    }
    fn empty() -> Self {
        // println!("DATETIME              {}", Utc.timestamp_opt(0, 0).unwrap());
        Self {
            id: GnuId::NONE,
            name: "".into(),
            tracker_addr: None,
            contact_url: "".into(),
            genre: "".into(),
            desc: "".into(),
            comment: "".into(),
            typee: "".into(),
            stream_type: "".into(),
            stream_ext: "".into(),
            bitrate: 0,
            number_of_listener: 0,
            number_of_relay: 0,
            created_at: Utc.timestamp_opt(0, 0).unwrap(),
            track: JsonTrack {
                title: "".into(),
                creator: "".into(),
                url: "".into(),
                album: "".into(),
                genre: "".into(),
            },
        }
    }
}

fn create_index_line(
    name: &String,
    id: &GnuId,
    tracker_addr: &Option<SocketAddr>,
    contact_url: &String,
    genre: &String,
    desc: &String,
    comment: &String,
    number_of_listener: i32,
    number_of_relay: i32,
    bitrate: i32,
    typee: &String,
    stream_type: &String,
    stream_ext: &String,
    created_at: &DateTime<Utc>,
) -> String {
    use html_escape::{encode_quoted_attribute, encode_safe};
    let diff_time = Utc::now() - created_at;
    let hour = diff_time.num_hours();
    let min = diff_time.num_minutes() % 60;

    let addr = tracker_addr
        .as_ref()
        .map(|a| a.to_string())
        .unwrap_or_default();

    format!(
        "{name}<>{id}<>{addr}<>{contact_url}<>{genre}<>{desc}<>{number_of_listener}<>{number_of_relay}<>{bitrate}<>{typee}<><><><><>{name_escaped}<>{time_hour}:{time_min:02}<>click<>{comment}<>0",
        name = encode_safe(&name.clone()),
        id = id,
        addr = addr,
        contact_url = encode_quoted_attribute(&contact_url),
        genre = encode_safe(&genre),
        desc = encode_safe(&desc),
        number_of_listener = number_of_listener,
        number_of_relay = number_of_relay,
        bitrate = bitrate,
        typee = encode_safe(&typee),
        name_escaped = encode_safe(&name),
        time_hour = hour,
        time_min = min,
        comment = comment
    )
}

impl From<IndexInfo> for JsonChannel {
    fn from(value: IndexInfo) -> Self {
        let mut j = JsonChannel::empty();
        let IndexInfo {
            id,
            name,
            tracker_addr,
            contact_url,
            genre,
            desc,
            comment,
            typee,
            stream_type,
            stream_ext,
            bitrate,
            number_of_listener,
            number_of_relay,
            created_at,
        } = value;
        j.id = id;
        j.typee = typee;
        j.name = name;
        j.tracker_addr = tracker_addr;
        j.contact_url = contact_url;
        j.genre = genre;
        j.desc = desc;
        j.comment = comment;
        j.stream_ext = stream_ext;
        j.bitrate = bitrate;
        j.number_of_listener = number_of_listener;
        j.number_of_relay = number_of_relay;
        j.created_at = created_at.unwrap_or_else(|| chrono::Utc::now());
        j
    }
}
