use axum::routing;

use crate::AppState;

mod pls;
mod stream;

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/pls/{channel_id}", routing::get(pls::pls_handler))
        .route("/stream/{channel_id_with_extention}", routing::get(stream::stream_handler))
    // TODO: /admin?cmd=viewxmlの実装
    // .route("/admin", routing::get(admin_handler))
    // ほかにもありそう
}

/*
async fn admin_handler(Query(_params): Query<AdminParams>) -> impl axum::response::IntoResponse {
    // <?xml version="1.0" encoding="utf-8"?>
    // <peercast session="EE22BBAD11B042BCA968128BC8735F9A">
    //   <servent uptime="361103" />
    //   <bandwidth in="2128" out="0" />
    //   <connections total="1" relays="0" direct="1" />
    //   <channels_relayed total="1">
    //     <channel id="73E669CBA92E7CA31143940B8DBFABBD" name="局長ch" bitrate="2128" comment="そこら辺の画像からでも生成できる・・・これがディープフェイクか" desc="18x AIエロ動画" genre="sp@@@game" type="FLV" url="https://jbbs.shitaraba.net/bbs/read.cgi/radio/25607/1766498555/" uptime="365" age="365" skip="0" bcflags="0">
    //       <hits hosts="0" listeners="0" relays="0" firewalled="0" closest="0" furthest="0" newest="0" />
    //       <relay listeners="1" relays="0" hosts="0" status="RECEIVE">
    //         <buffer bytes="812995" duration="2.977745" />
    //       </relay>
    //       <track title="" album="" genre="" artist="" contact="" />
    //     </channel>
    //   </channels_relayed>
    //   <channels_found total="1">
    //     <channel id="73E669CBA92E7CA31143940B8DBFABBD" name="局長ch" bitrate="2128" comment="そこら辺の画像からでも生成できる・・・これがディープフェイクか" desc="18x AIエロ動画" genre="sp@@@game" type="FLV" url="https://jbbs.shitaraba.net/bbs/read.cgi/radio/25607/1766498555/" uptime="365" age="365" skip="0" bcflags="0">
    //       <hits hosts="0" listeners="0" relays="0" firewalled="0" closest="0" furthest="0" newest="0" />
    //       <relay listeners="1" relays="0" hosts="0" status="RECEIVE">
    //         <buffer bytes="812995" duration="2.977745" />
    //       </relay>
    //       <track title="" album="" genre="" artist="" contact="" />
    //     </channel>
    //   </channels_found>
    // </peercast>
    "".into_response()
} */
