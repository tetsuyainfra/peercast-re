use std::i32;

// use peercast_root::{PortLevel, RestrictPortLevel};

use crate::{RestrictPortLevel, model::PortLevel, model::JsonChannel};

/// Vec<JsonChannel>を条件についてフィルタリングす
/// @param namespace: YPの名前空間
/// @param yp_listener_hideable: Listener数を非表示にするか
/// @param yp_port_check_level: Portcheckのレベル制限
/// @param yp_limit_speed: Portcheckで制限する速度(KBps単位)
/// @param own_port_level: 自分のポートレベル
/// @param own_speed: 自分の速度(KBps単位)
/// @param channels: フィルタリング対象のチャンネルリスト
///
/// @return: フィルタリング後のチャンネルリスト
#[inline]
fn filter_channel(
    // Config
    namespace: &str,
    yp_listener_hideable: bool,
    yp_restrict_port_level: RestrictPortLevel,
    yp_restrict_speed: u32,
    own_port_level: PortLevel,
    // Input
    channel: JsonChannel,
) -> Option<JsonChannel> {
    let mut c = channel;
    debug_assert!(crate::YP_LIMIT_SPEED_MIN < yp_restrict_speed); // MEMO: testの時消えなければよい

    // 名前空間のフィルタリング
    let mut c = if let Some(stripped_ns_genre) = c.genre.strip_prefix(&namespace) {
        if let Some(stripped_hide_genre) = stripped_ns_genre.strip_prefix("?") {
            // 名前空間がypで、?がついていて、かつypの設定が有効な時はListener数を非表示にする
            if yp_listener_hideable {
                c.number_of_relay = -1;
                c.number_of_listener = -1;
            }
            c.genre = stripped_hide_genre.to_string();
        } else {
            // 名前空間がypで、?がついていない場合はListener数を表示する
            c.genre = stripped_ns_genre.to_string();
        }
        Some(c)
    } else {
        // 名前空間が一致しない場合はフィルタリング
        None
    }?;

    // ポートチェック制限を次の通りに変換し、数値比較できるようにする
    // None(ポートチェック無し)=0
    // PortCheck=1
    // BroadcastSpeed = Channel.bitrate
    // RestrictSpeed = yp_restrict_speed

    // YPで設定しているポートチェック制限
    let max_speed = match yp_restrict_port_level {
        RestrictPortLevel::None => 0,
        RestrictPortLevel::PortCheck => 1,
        RestrictPortLevel::BroadcastSpeed => c.bitrate.clamp(0, i32::MAX) as u32,
        RestrictPortLevel::RestrictSpeed => yp_restrict_speed,
    };
    // Channel(Genre)で設定しているポートチェック制限
    let ch_wanted_speed_limit = if let Some(g) = c.genre.strip_prefix("@@@") {
        // YPのアップロード速度
        c.genre = g.to_string();
        yp_restrict_speed
    } else if let Some(g) = c.genre.strip_prefix("@@") {
        // 配信のビットレート速度
        c.genre = g.to_string();
        c.bitrate.clamp(0, i32::MAX) as u32
    } else if let Some(g) = c.genre.strip_prefix("@") {
        // ポートチェックあり
        c.genre = g.to_string();
        1
    } else {
        // ポートチェックしない
        0
    };

    // [0, ch_wanted_speed_limit, max_speed]をソートし、2番目の要素から適切な制限レベルを取得する
    // ※ clampでもよい
    let calculated_limit = ch_wanted_speed_limit.clamp(0, max_speed);

    fn to_speed(port_level: PortLevel) -> u32 {
        match port_level {
            PortLevel::Incomplete => 0,
            PortLevel::None => 0,
            PortLevel::Welldone => 1,
            // PortLevel::WelldoneWithSpeed(speed) => speed as u32,
            PortLevel::WelldoneWithSpeed => 2,
        }
    }

    // 自分のポート速度を取得
    let own_speed = to_speed(own_port_level);
    if own_speed < calculated_limit {
        // ポートレベルが制限よりも低い場合はフィルタリング
        c.tracker_addr = None;
        match calculated_limit {
            0 => {
                unreachable!("RestrectedPortLevel::None should not be used here");
            }
            1 => {
                // ポートチェックあり
                c.genre = format!("{}: {}", "要ポート開放", c.genre);
            }
            _ => {
                // 配信ビットレートで表示制限
                c.genre = format!("{}({}Kbps): {}", "要制限速度", calculated_limit, c.comment);
            }
        }
    }

    Some(c)
}
pub fn filter_channels(
    // Config
    namespace: &str,
    yp_listener_hideable: bool,
    yp_restrict_port_level: RestrictPortLevel,
    yp_restrict_speed: u32,
    own_port_level: PortLevel,
    _own_speed: u32,
    // Input
    channel: Vec<JsonChannel>,
) -> Vec<JsonChannel> {
    channel
        .into_iter()
        .filter_map(|c| {
            filter_channel(
                namespace,
                yp_listener_hideable,
                yp_restrict_port_level,
                yp_restrict_speed,
                own_port_level,
                c,
            )
        })
        .collect()
}

/*
TODO: テスト書き直そう

//-------------------------------------------------------------------------------
// Test
//-------------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use serde_json::de;

    use super::*;
    use crate::api::JsonChannel;

    fn jc_empty(genre: &str) -> JsonChannel {
        let mut c = JsonChannel::empty();
        c.genre = genre.into();
        c
    }

    fn jc_ip(genre: &str) -> JsonChannel {
        let mut c = jc_empty(genre);
        c.tracker_addr = Some("192.168.0.1:7144".parse().unwrap());
        c
    }

    fn jc_speed(genre: &str, speed: i32) -> JsonChannel {
        let mut c = jc_ip(genre);
        c.bitrate = speed as i32;
        c
    }

    /// Namespceのフィルタリングテスト
    #[test]
    fn test_filter_channels_namespace() {
        fn filter_ch(yp_namespace: &str, ch: JsonChannel) -> Option<JsonChannel> {
            filter_channel(
                yp_namespace,
                false,
                RestrictPortLevel::None,
                500,
                PortLevel::None,
                ch,
            )
        };
        let c = filter_ch("", jc_empty(""));
        let c = c.unwrap();
        assert_eq!(c.genre, "");

        let c = filter_ch("", jc_empty("Genre"));
        let c = c.unwrap();
        assert_eq!(c.genre, "Genre");

        let c = filter_ch("yp", jc_empty("Genre"));
        assert!(c.is_none());

        let c = filter_ch("p", jc_empty("ypGenre"));
        assert!(c.is_none());
    }

    // リスナー数の非表示テスト
    #[test]
    fn test_filter_channels_hideable() {
        fn filter_ch(
            yp_namespace: &str,
            yp_listener_hideable: bool,
            ch: JsonChannel,
        ) -> Option<JsonChannel> {
            filter_channel(
                yp_namespace,
                yp_listener_hideable,
                RestrictPortLevel::None,
                500,
                PortLevel::None,
                ch,
            )
        };

        // hideable: False, genre_?_include: False
        let c = filter_ch("yp", false, jc_empty("ypGenre"));
        let c = c.unwrap();
        assert_eq!(c.genre, "Genre");
        assert_eq!(c.number_of_listener, 0);
        assert_eq!(c.number_of_relay, 0);

        // hideable: False, genre_?_include: True
        let c = filter_ch("yp", false, jc_empty("yp?Genre"));
        let c = c.unwrap();
        assert_eq!(c.genre, "Genre");
        assert_eq!(c.number_of_listener, 0);
        assert_eq!(c.number_of_relay, 0);

        // hideable: True, genre_?_include: false
        let c = filter_ch("yp", true, jc_empty("ypGenre"));
        let c = c.unwrap();
        assert_eq!(c.genre, "Genre");
        assert_eq!(c.number_of_listener, 0);
        assert_eq!(c.number_of_relay, 0);

        // hideable: True, genre_?_include: True
        let c = filter_ch("yp", true, jc_empty("yp?Genre"));
        let c = c.unwrap();
        assert_eq!(c.genre, "Genre");
        assert_eq!(c.number_of_listener, -1);
        assert_eq!(c.number_of_relay, -1);
    }

    ///
    /// ポート開放制限 "なし"
    ///
    #[test]
    fn test_filter_channels_portlimit_0() {
        fn filter_ch(own_port_level: PortLevel, ch: JsonChannel) -> Option<JsonChannel> {
            filter_channel("yp",
             true,
             RestrictPortLevel::None,
             500,
             own_port_level,
             ch)
        }

        #[rustfmt::skip]
        let cases = [
            ((PortLevel::None, "yp?Genre"),                         true, "Genre"),
            ((PortLevel::Welldone, "yp?Genre"),                     true, "Genre"),
            ((PortLevel::WelldoneWithSpeed(1000), "yp?Genre"),      true, "Genre"),
            ((PortLevel::WelldoneWithSpeed(2000), "yp?Genre"),      true, "Genre"),
            //
            ((PortLevel::None, "yp?@Genre"),                         true, "Genre"),
            ((PortLevel::Welldone, "yp?@Genre"),                     true, "Genre"),
            ((PortLevel::WelldoneWithSpeed(1000), "yp?@Genre"),      true, "Genre"),
            ((PortLevel::WelldoneWithSpeed(2000), "yp?@Genre"),      true, "Genre"),
            //
            ((PortLevel::None, "yp?@@Genre"),                         true, "Genre"),
            ((PortLevel::Welldone, "yp?@@Genre"),                     true, "Genre"),
            ((PortLevel::WelldoneWithSpeed(1000), "yp?@@Genre"),      true, "Genre"),
            ((PortLevel::WelldoneWithSpeed(2000), "yp?@@Genre"),      true, "Genre"),
            //
            ((PortLevel::None, "yp?@@@Genre"),                         true, "Genre"),
            ((PortLevel::Welldone, "yp?@@@Genre"),                     true, "Genre"),
            ((PortLevel::WelldoneWithSpeed(1000), "yp?@@@Genre"),      true, "Genre"),
            ((PortLevel::WelldoneWithSpeed(2000), "yp?@@@Genre"),      true, "Genre"),
            //
            ((PortLevel::None, "yp?@@@@Genre"),                         true, "@Genre"),
            ((PortLevel::Welldone, "yp?@@@@Genre"),                     true, "@Genre"),
            ((PortLevel::WelldoneWithSpeed(1000), "yp?@@@@Genre"),      true, "@Genre"),
            ((PortLevel::WelldoneWithSpeed(2000), "yp?@@@@Genre"),      true, "@Genre"),
        ];

        for (input, is_addr_exists, ret_genre) in cases {
            let (own_port_level, genre) = input;
            println!("{own_port_level:?}, {genre}");

            // ポート開放はチャンネル情報の改変を行うだけなので必ずチャンネルは存在する
            let c = filter_ch(own_port_level, jc_ip(genre)).unwrap();
            assert_eq!(c.tracker_addr.is_some(), is_addr_exists);
            assert_eq!(c.genre, ret_genre);
        }
    }

    ///
    /// ポート開放制限 "あり"
    ///
    #[test]
    fn test_filter_channels_portlimit_1() {
        fn filter_ch(own_port_level: PortLevel, ch: JsonChannel) -> Option<JsonChannel> {
            filter_channel(
                "yp",
                true,
                RestrictPortLevel::PortCheck,
                500,
                own_port_level,
                ch,
            )
        }

        let c0 = jc_speed("yp?Genre", 1500);
        let c1 = jc_speed("yp?@Genre", 1500);
        let c2 = jc_speed("yp?@@Genre", 1500);
        let c3 = jc_speed("yp?@@@Genre", 1500);
        let c4 = jc_speed("yp?@@@@Genre", 1500);
        #[rustfmt::skip]
        let cases = [
            // チェック無し
            ((PortLevel::None, c0.clone()),                         true, "Genre"),
            ((PortLevel::Welldone, c0.clone()),                     true, "Genre"),
            ((PortLevel::WelldoneWithSpeed(1000), c0.clone()),      true, "Genre"),
            ((PortLevel::WelldoneWithSpeed(2000), c0.clone()),      true, "Genre"),
            // ポート開放チェック
            ((PortLevel::None, c1.clone()),                         false, "要ポート開放: Genre"),
            ((PortLevel::Welldone, c1.clone()),                     true,  "Genre"),
            ((PortLevel::WelldoneWithSpeed(1000), c1.clone()),      true,  "Genre"),
            ((PortLevel::WelldoneWithSpeed(2000), c1.clone()),      true,  "Genre"),
            // 配信ビットレート制限
            ((PortLevel::None, c2.clone()),                         false, "要ポート開放: Genre"),
            ((PortLevel::Welldone, c2.clone()),                     true,  "Genre"),
            ((PortLevel::WelldoneWithSpeed(1000), c2.clone()),      true,  "Genre"),
            ((PortLevel::WelldoneWithSpeed(2000), c2.clone()),      true,  "Genre"),
            // アップロード制限
            ((PortLevel::None, c3.clone()),                         false, "要ポート開放: Genre"),
            ((PortLevel::Welldone, c3.clone()),                     true,  "Genre"),
            ((PortLevel::WelldoneWithSpeed(1000), c3.clone()),      true,  "Genre"),
            ((PortLevel::WelldoneWithSpeed(2000), c3.clone()),      true,  "Genre"),
            ((PortLevel::WelldoneWithSpeed(2999), c3.clone()),      true,  "Genre"),
            ((PortLevel::WelldoneWithSpeed(3000), c3.clone()),      true,  "Genre"),
        ];

        for (input, is_addr_exists, ret_genre) in cases {
            let (own_port_level, c) = input;
            println!("{own_port_level:?}, {}", c.genre);
            let c = filter_ch(own_port_level, c).unwrap();
            assert_eq!(c.tracker_addr.is_some(), is_addr_exists);
            assert_eq!(c.genre, ret_genre);
        }
    }


    ///
    /// ポート制限あり(配信ビットレート制限)のテスト
    ///
    #[test]
    fn test_filter_channels_portlimit_2() {
        fn filter_ch(own_port_level: PortLevel, ch: JsonChannel) -> Option<JsonChannel> {
            filter_channel(
                "yp",
                true,
                RestrictPortLevel::BroadcastSpeed,
                1500,
                own_port_level,
                ch,
            )
        }

        // MEMO
        // 要速度制限解除(2600Kbps): ってのがよいのでは？
        //
        let c0 = jc_speed("yp?Genre", 1500);
        let c1 = jc_speed("yp?@Genre", 1500);
        let c2 = jc_speed("yp?@@Genre", 1500);
        let c3 = jc_speed("yp?@@@Genre", 1500);
        let c4 = jc_speed("yp?@@@@Genre", 1500);
        #[rustfmt::skip]
        let cases = [
            // チェック無し
            ((PortLevel::None, c0.clone()),                         true, "Genre"),
            ((PortLevel::Welldone, c0.clone()),                     true, "Genre"),
            ((PortLevel::WelldoneWithSpeed(1000), c0.clone()),      true, "Genre"),
            ((PortLevel::WelldoneWithSpeed(2000), c0.clone()),      true, "Genre"),
            // ポート開放チェック
            ((PortLevel::None, c1.clone()),                         false, "要ポート開放: Genre"),
            ((PortLevel::Welldone, c1.clone()),                     true,  "Genre"),
            ((PortLevel::WelldoneWithSpeed(1000), c1.clone()),      true,  "Genre"),
            ((PortLevel::WelldoneWithSpeed(2000), c1.clone()),      true,  "Genre"),
            // 配信ビットレート制限
            ((PortLevel::None, c2.clone()),                         false, "要ポート開放: Genre"),
            ((PortLevel::Welldone, c2.clone()),                     false,  "Genre"),
            ((PortLevel::WelldoneWithSpeed(1000), c2.clone()),      false,  "Genre"),
            ((PortLevel::WelldoneWithSpeed(2000), c2.clone()),      true,  "Genre"),
            // アップロード制限
            ((PortLevel::None, c3.clone()),                         false, "要ポート開放: Genre"),
            ((PortLevel::Welldone, c3.clone()),                     false,  "Genre"),
            ((PortLevel::WelldoneWithSpeed(1000), c3.clone()),      false,  "Genre"),
            ((PortLevel::WelldoneWithSpeed(2000), c3.clone()),      true,  "Genre"),
            ((PortLevel::WelldoneWithSpeed(2999), c3.clone()),      true,  "Genre"),
            ((PortLevel::WelldoneWithSpeed(3000), c3.clone()),      true,  "Genre"),
        ];

        for (input, is_addr_exists, ret_genre) in cases {
            let (own_port_level, c) = input;
            println!("{own_port_level:?}, {}", c.genre);
            let c = filter_ch(own_port_level, c).unwrap();
            assert_eq!(c.tracker_addr.is_some(), is_addr_exists);
            assert_eq!(c.genre, ret_genre);
        }
    }


    // ポート制限あり(アップロード速度制限)のテスト
    #[test]
    fn test_filter_channels_portlimit_3() {
        fn filter_ch(own_port_level: PortLevel, ch: JsonChannel) -> Option<JsonChannel> {
            filter_channel(
                "yp",
                true,
                RestrictPortLevel::PortCheck,
                3000,
                own_port_level,
                ch,
            )
        }

        #[rustfmt::skip]
        let c0 = jc_speed("yp?Genre", 1500);
        let c1 = jc_speed("yp?@Genre", 1500);
        let c2 = jc_speed("yp?@@Genre", 1500);
        let c3 = jc_speed("yp?@@@Genre", 1500);
        let c4 = jc_speed("yp?@@@@Genre", 1500);

        let cases = [
            // チェック無し
            ((PortLevel::None, c0.clone()),                         true, "要ポート開放: Genre"),
            ((PortLevel::Welldone, c0.clone()),                     true, "要ポート開放: Genre"),
            ((PortLevel::WelldoneWithSpeed(1000), c0.clone()),      true, "要ポート開放: Genre"),
            ((PortLevel::WelldoneWithSpeed(2000), c0.clone()),      true, "要ポート開放: Genre"),
            // ポート開放チェック
            ((PortLevel::None, c1.clone()),                         false, "要速度制限(): Genre"),
            ((PortLevel::Welldone, c1.clone()),                     true,  "Genre"),
            ((PortLevel::WelldoneWithSpeed(1000), c1.clone()),      true,  "Genre"),
            ((PortLevel::WelldoneWithSpeed(2000), c1.clone()),      true,  "Genre"),
            // 配信ビットレート制限
            ((PortLevel::None, c2.clone()),                         false, "Genre"),
            ((PortLevel::Welldone, c2.clone()),                     false, "Genre"),
            ((PortLevel::WelldoneWithSpeed(1000), c2.clone()),      false, "Genre"),
            ((PortLevel::WelldoneWithSpeed(2000), c2.clone()),      true,  "Genre"),
            // アップロード制限
            ((PortLevel::None, c3.clone()),                         false, "Genre"),
            ((PortLevel::Welldone, c3.clone()),                     false, "Genre"),
            ((PortLevel::WelldoneWithSpeed(1000), c3.clone()),      false, "Genre"),
            ((PortLevel::WelldoneWithSpeed(2000), c3.clone()),      false, "Genre"),
            ((PortLevel::WelldoneWithSpeed(2999), c3.clone()),      false,  "Genre"),
            ((PortLevel::WelldoneWithSpeed(3000), c3.clone()),      true,  "Genre"),
            // アップロード制限
            // ((PortLevel::None, c4.clone()),                         true, "@Genre"),
            // ((PortLevel::Welldone, c4.clone()),                     true, "@Genre"),
            // ((PortLevel::WelldoneWithSpeed(1000), c4.clone()),      true, "@Genre"),
            // ((PortLevel::WelldoneWithSpeed(2000), c4.clone()),      true, "@Genre"),
        ];

        for (input, is_addr_exists, ret_genre) in cases {
            println!("{input:?}");
            let (own_port_level, c) = input;
            let c = filter_ch(own_port_level, c).unwrap();
            assert_eq!(c.tracker_addr.is_some(), is_addr_exists);
            assert_eq!(c.genre, ret_genre);
        }
    }
}

*/
