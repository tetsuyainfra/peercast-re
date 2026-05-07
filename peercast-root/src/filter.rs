// use std::i32;

// // use peercast_root::{PortLevel, RestrictPortLevel};

// use crate::{RestrictPortLevel, model::ChannelMeta, model::PortLevel};

// /// Vec<JsonChannel>を条件についてフィルタリングす
// /// @param namespace: YPの名前空間
// /// @param yp_listener_hideable: Listener数を非表示にするか
// /// @param yp_port_check_level: Portcheckのレベル制限
// /// @param yp_limit_speed: Portcheckで制限する速度(KBps単位)
// /// @param own_port_level: 自分のポートレベル
// /// @param own_speed: 自分の速度(KBps単位)
// /// @param channels: フィルタリング対象のチャンネルリスト
// ///
// /// @return: フィルタリング後のチャンネルリスト
// #[inline]
// fn filter_channel(
//     // Config
//     namespace: &str,
//     yp_listener_hideable: bool,
//     yp_restrict_port_level: RestrictPortLevel,
//     yp_restrict_speed: u32,
//     own_port_level: PortLevel,
//     // Input
//     channel_meta: ChannelMeta,
// ) -> Option<ChannelMeta> {
//     let mut c = channel_meta;
//     debug_assert!(crate::YP_LIMIT_SPEED_MIN < yp_restrict_speed); // MEMO: testの時消えなければよい

//     // 名前空間のフィルタリング
//     let mut c = if let Some(stripped_ns_genre) = c.genre.strip_prefix(&namespace) {
//         if let Some(stripped_hide_genre) = stripped_ns_genre.strip_prefix("?") {
//             // 名前空間がypで、?がついていて、かつypの設定が有効な時はListener数を非表示にする
//             if yp_listener_hideable {
//                 c.number_of_relay = -1;
//                 c.number_of_listener = -1;
//             }
//             c.genre = stripped_hide_genre.to_string();
//         } else {
//             // 名前空間がypで、?がついていない場合はListener数を表示する
//             c.genre = stripped_ns_genre.to_string();
//         }
//         Some(c)
//     } else {
//         // 名前空間が一致しない場合はフィルタリング
//         None
//     }?;

//     // ポートチェック制限を次の通りに変換し、数値比較できるようにする
//     // None(ポートチェック無し)=0
//     // PortCheck=1
//     // BroadcastSpeed = Channel.bitrate
//     // RestrictSpeed = yp_restrict_speed

//     // YPで設定しているポートチェック制限
//     let max_speed = match yp_restrict_port_level {
//         RestrictPortLevel::None => 0,
//         RestrictPortLevel::PortCheck => 1,
//         RestrictPortLevel::BroadcastSpeed => c.bitrate.clamp(0, i32::MAX) as u32,
//         RestrictPortLevel::RestrictSpeed => yp_restrict_speed,
//     };
//     // Channel(Genre)で設定しているポートチェック制限
//     let ch_wanted_speed_limit = if let Some(g) = c.genre.strip_prefix("@@@") {
//         // YPのアップロード速度
//         c.genre = g.to_string();
//         yp_restrict_speed
//     } else if let Some(g) = c.genre.strip_prefix("@@") {
//         // 配信のビットレート速度
//         c.genre = g.to_string();
//         c.bitrate.clamp(0, i32::MAX) as u32
//     } else if let Some(g) = c.genre.strip_prefix("@") {
//         // ポートチェックあり
//         c.genre = g.to_string();
//         1
//     } else {
//         // ポートチェックしない
//         0
//     };

//     // [0, ch_wanted_speed_limit, max_speed]をソートし、2番目の要素から適切な制限レベルを取得する
//     // ※ clampでもよい
//     let calculated_limit = ch_wanted_speed_limit.clamp(0, max_speed);

//     fn to_speed(port_level: PortLevel) -> u32 {
//         match port_level {
//             PortLevel::Incomplete => 0,
//             PortLevel::Welldone => 1,
//         }
//     }

//     // 自分のポート速度を取得
//     let own_speed = to_speed(own_port_level);
//     if own_speed < calculated_limit {
//         // ポートレベルが制限よりも低い場合はフィルタリング
//         c.tracker_addr = None;
//         match calculated_limit {
//             0 => {
//                 unreachable!("RestrectedPortLevel::None should not be used here");
//             }
//             1 => {
//                 // ポートチェックあり
//                 c.genre = format!("{}: {}", "要ポート開放", c.genre);
//             }
//             _ => {
//                 // 配信ビットレートで表示制限
//                 c.genre = format!("{}({}Kbps): {}", "要制限速度", calculated_limit, c.comment);
//             }
//         }
//     }

//     Some(c)
// }
// pub fn filter_channels(
//     // Config
//     namespace: &str,
//     yp_listener_hideable: bool,
//     yp_restrict_port_level: RestrictPortLevel,
//     yp_restrict_speed: u32,
//     own_port_level: PortLevel,
//     _own_speed: u32,
//     // Input
//     channel_metas: Vec<ChannelMeta>,
// ) -> Vec<ChannelMeta> {
//     channel_metas
//         .into_iter()
//         .filter_map(|c| {
//             filter_channel(
//                 namespace,
//                 yp_listener_hideable,
//                 yp_restrict_port_level,
//                 yp_restrict_speed,
//                 own_port_level,
//                 c,
//             )
//         })
//         .collect()
// }
