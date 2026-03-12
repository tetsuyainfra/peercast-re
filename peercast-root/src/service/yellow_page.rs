use std::fmt;

use chrono::Utc;

use crate::{
    RestrictPortLevel,
    model::{ChannelMeta, CheckedHost, IndexInfo, PortLevel},
    utils::process_uptime,
};

#[derive(Debug, Clone)]
pub struct SiteConfig {
    /// YPの名前
    pub yp_name: String,

    /// リスナー数非表示が可能か
    pub listener_hideable: bool,

    /// 制限速度
    pub restrict_speed: u32,

    /// 設定できるポートチェックのレベル
    pub max_restrict_level: RestrictPortLevel,
}

type YpStatusFunc = Box<dyn Fn(&CheckedHost) -> Option<ChannelMeta> + Send + Sync>;
// type YpSystemStatusFunc = fn(host: &CheckedHost) -> Option<ChannelMeta>;
type YpSystemStatusFunc = Box<dyn Fn(&CheckedHost) -> Option<ChannelMeta> + Send + Sync>;

pub struct YellowPageService {
    config: SiteConfig,
    footer_channels: Vec<IndexInfo>,
    create_user_status_func: YpStatusFunc,
    create_status_channel: YpSystemStatusFunc,
}

impl std::fmt::Debug for YellowPageService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("YellowPageService")
            .field("config", &self.config)
            .field("footer_channels", &self.footer_channels)
            .field("create_status_channel", &"<function>")
            .finish()
    }
}

impl YellowPageService {
    pub fn new(site_config: SiteConfig) -> Self {
        Self {
            config: site_config,
            footer_channels: Vec::new(),
            create_user_status_func: Box::new(|_| None),
            create_status_channel: Box::new(|_| None),
        }
    }
    pub fn add_footer_channels(mut self, index_infos: Vec<IndexInfo>) -> Self {
        self.footer_channels.reserve(index_infos.len());
        self.footer_channels.extend(index_infos.into_iter());
        self
    }
    pub fn add_create_user_status_func(mut self, create_user_status_func: YpSystemStatusFunc) -> Self {
        self.create_user_status_func = create_user_status_func;
        self
    }
    pub fn add_create_status_channel_func(mut self, create_status_channel_func: YpSystemStatusFunc) -> Self {
        self.create_status_channel = create_status_channel_func;
        self
    }
}

impl YellowPageService {
    const MSG_PORT_CHECK: &'static str = "- [要ポート開放]";
    const MSG_UPLOAD_NEEDS_TEST: &'static str = "- [要帯域測定]";
    const MSG_UPLOAD_NOT_EXCEED_LIMIT: &'static str = "- [帯域不足]";

    pub fn filter_channel_meta(&self, host: &CheckedHost, channels: Vec<ChannelMeta>) -> Vec<ChannelMeta> {
        let filtered_channels = Self::filter_channels(&self.config, &host, channels);
        let mut channels = Self::merge_footer(&self.footer_channels, filtered_channels);
        if let Some(channel_status) = (self.create_user_status_func)(host) {
            channels.push(channel_status)
        }
        if let Some(channel_status) = (self.create_status_channel)(host) {
            channels.push(channel_status)
        }

        channels
    }

    fn filter_channels(config: &SiteConfig, host: &CheckedHost, channels: Vec<ChannelMeta>) -> Vec<ChannelMeta> {
        // TODO: implement filtering logic
        let SiteConfig {
            yp_name,
            listener_hideable,
            max_restrict_level,
            restrict_speed,
        } = config;

        let yp_name_len = yp_name.len();

        let channels = channels
            .into_iter()
            .filter_map(|mut ch| {
                // [yp_name]から始まってるかチェック
                if !ch.genre.starts_with(yp_name) {
                    return None;
                }
                let mut rest = &ch.genre[yp_name_len..];

                // listener非表示
                let mut hide_listener = false;
                if rest.starts_with('?') {
                    hide_listener = true;
                    rest = &rest[1..];
                }
                if *listener_hideable && hide_listener {
                    ch.number_of_listener = -1;
                    ch.number_of_relay = -1;
                }

                // @数 = Level
                let at_count = rest.chars().take_while(|c| *c == '@').count();
                rest = &rest[at_count..];
                // サイトで設定された最大値に置き換え
                let limit_level: RestrictPortLevel = at_count.clamp(0, *max_restrict_level as usize).into();

                ch.display_genre = Some(rest.into());

                // ---- 制限チェック ----
                match limit_level {
                    RestrictPortLevel::None => {}

                    // @ → ポートチェック
                    RestrictPortLevel::PortCheck => {
                        if host.port_level != PortLevel::Welldone {
                            Self::convert_meta(&mut ch, Self::MSG_PORT_CHECK);
                        }
                    }

                    // @@ → ポート + 配信ビットレート制限
                    RestrictPortLevel::BroadcastSpeed => {
                        if host.port_level != PortLevel::Welldone {
                            Self::convert_meta(&mut ch, Self::MSG_PORT_CHECK);
                        }

                        if None == host.upload_speed {
                            Self::convert_meta(&mut ch, Self::MSG_UPLOAD_NEEDS_TEST);
                        }
                        if let Some(host_speed) = host.upload_speed {
                            if host_speed < ch.bitrate.max(0) as u32 {
                                Self::convert_meta(&mut ch, Self::MSG_UPLOAD_NOT_EXCEED_LIMIT);
                            }
                        }
                    }

                    // @@@ → ポート + YP指定帯域制限
                    RestrictPortLevel::RestrictSpeed => {
                        if host.port_level != PortLevel::Welldone {
                            Self::convert_meta(&mut ch, Self::MSG_PORT_CHECK);
                        }

                        if None == host.upload_speed {
                            Self::convert_meta(&mut ch, Self::MSG_UPLOAD_NEEDS_TEST);
                        }
                        if let Some(host_speed) = host.upload_speed {
                            if host_speed < *restrict_speed {
                                Self::convert_meta(&mut ch, Self::MSG_UPLOAD_NOT_EXCEED_LIMIT);
                            }
                        }
                    }
                };

                Some(ch)
            })
            .collect();

        channels
    }

    fn merge_footer(footer_channels: &Vec<IndexInfo>, mut channels: Vec<ChannelMeta>) -> Vec<ChannelMeta> {
        channels.reserve(footer_channels.len());
        channels.extend(footer_channels.iter().map(|e| e.into()));
        channels
    }

    fn convert_meta(ch: &mut ChannelMeta, msg: &'static str) {
        // Trackerアドレスを隠す
        ch.tracker_addr = None;

        ch.desc.push_str(msg);
    }
}

/// YpAppendUserStatus::Defaultの関数を作成する
#[allow(non_snake_case)]
pub fn createUserStatusDefaultFunction(config: &SiteConfig) -> YpSystemStatusFunc {
    let name = format!("{}◆UserStatus", config.yp_name.to_ascii_uppercase());

    let f = move |host: &CheckedHost| -> Option<ChannelMeta> {
        let mut meta = ChannelMeta::Empty();
        let now = Utc::now();
        let speed_args = match host.upload_speed {
            Some(s) => format_args!("{} Kbps", s.clone()),
            None => format_args!("Unknown"),
        };
        //
        meta.name = name.clone();
        meta.genre = format!("");
        meta.display_genre = None;
        meta.comment =
            format!("host={}:{} port={} speed={}", host.ip_address.0, host.port, host.port_level, speed_args);
        meta.number_of_listener = -8;
        meta.number_of_relay = -8;
        meta.typee = String::from("RAW");
        meta.created_at = now;

        Some(meta)
    };

    Box::new(f)
}

/// YpAppendSystemStatus::Defaultの関数を作成する
#[allow(non_snake_case)]
pub fn createSystemStatusDefaultFunction(config: &SiteConfig) -> YpSystemStatusFunc {
    let name = format!("{}◆Status", config.yp_name.to_ascii_uppercase());

    let f = move |_host: &CheckedHost| -> Option<ChannelMeta> {
        let mut meta = ChannelMeta::Empty();
        let now = Utc::now();
        let uptime = process_uptime();
        //
        meta.name = name.clone();
        meta.genre = format!("");
        meta.display_genre = None;
        meta.comment =
            format!("ProcessUptime={} Updated={}", uptime, now.to_rfc3339_opts(chrono::SecondsFormat::Secs, false));
        meta.number_of_listener = -9;
        meta.number_of_relay = -9;
        meta.typee = String::from("RAW");
        meta.created_at = now;

        Some(meta)
    };

    Box::new(f)
}

/// YpAppendSystemStatus::WithStatusの関数を作成する
#[allow(non_snake_case)]
pub fn createSystemStatusWithHostInfoFunction(config: &SiteConfig) -> YpSystemStatusFunc {
    let name = format!("{}◆Status", config.yp_name.to_ascii_uppercase());

    let f = move |host: &CheckedHost| -> Option<ChannelMeta> {
        let mut meta = ChannelMeta::Empty();
        let now = Utc::now();
        let uptime = process_uptime();
        let speed_args = match host.upload_speed {
            Some(s) => format_args!("{} Kbps", s.clone()),
            None => format_args!("Unknown"),
        };

        meta.name = name.clone();
        meta.genre = format!("");
        meta.display_genre = None;
        meta.comment = format!(
            "ProcessUptime={} Updated={} host={}:{} port={} speed={}",
            uptime,
            now.to_rfc3339_opts(chrono::SecondsFormat::Secs, false),
            host.ip_address.0,
            host.port,
            host.port_level,
            speed_args
        );
        meta.number_of_listener = -9;
        meta.number_of_relay = -9;
        meta.typee = String::from("RAW");
        meta.created_at = now;

        Some(meta)
    };

    Box::new(f)
}

#[cfg(test)]
mod test {
    use crate::model::DbIpAddr;
    use chrono::Utc;

    use super::*;

    fn checkd_host() -> CheckedHost {
        CheckedHost {
            id: Some(1),
            ip_address: DbIpAddr("10.10.10.10".parse().unwrap()),
            port: 7144,
            port_level: PortLevel::Welldone,
            upload_speed: Some(1000),
            updated_at: Utc::now(),
        }
    }

    fn close_checkd_host() -> CheckedHost {
        let mut host = checkd_host();
        host.port_level = PortLevel::Incomplete;
        host
    }

    /// ポートが開いてないホスト
    #[test]
    fn test_yp_closed_host() {
        let yp = YellowPageService::new(SiteConfig {
            yp_name: "yp".to_string(),
            listener_hideable: true,
            max_restrict_level: RestrictPortLevel::RestrictSpeed,
            restrict_speed: 2000,
        });
        {
            let host = close_checkd_host();
            let mut channel = ChannelMeta::Empty();
            channel.tracker_addr = Some("127.0.0.1:7144".parse().unwrap());
            channel.number_of_listener = 100;
            channel.number_of_relay = 100;
            channel.genre = "yp?ジャンル".to_string();

            let channels = yp.filter_channel_meta(&host, vec![channel.clone()]);
            assert_eq!(channels.len(), 1);
            assert_eq!(channels[0].tracker_addr.is_some(), true);
            assert_eq!(channels[0].display_genre.as_ref().unwrap(), "ジャンル");
            assert_eq!(channels[0].number_of_listener, -1);
            assert_eq!(channels[0].number_of_relay, -1);

            channel.genre = "yp?@ジャンル".to_string();
            let channels = yp.filter_channel_meta(&host, vec![channel.clone()]);
            assert_eq!(channels[0].tracker_addr.is_none(), true);
            assert_eq!(channels[0].display_genre.as_ref().unwrap(), "ジャンル");
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_PORT_CHECK), true);

            channel.genre = "yp?@@ジャンル".to_string();
            let channels = yp.filter_channel_meta(&host, vec![channel.clone()]);
            assert_eq!(channels[0].tracker_addr.is_none(), true);
            assert_eq!(channels[0].display_genre.as_ref().unwrap(), "ジャンル");
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_PORT_CHECK), true);

            channel.genre = "yp?@@@ジャンル".to_string();
            let channels = yp.filter_channel_meta(&host, vec![channel.clone()]);
            assert_eq!(channels[0].tracker_addr.is_none(), true);
            assert_eq!(channels[0].display_genre.as_ref().unwrap(), "ジャンル");
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_PORT_CHECK), true);

            channel.genre = "yp?@@@@ジャンル".to_string();
            let channels = yp.filter_channel_meta(&host, vec![channel.clone()]);
            assert_eq!(channels[0].tracker_addr.is_none(), true);
            assert_eq!(channels[0].display_genre.as_ref().unwrap(), "ジャンル");
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_PORT_CHECK), true);
        }
    }

    /// ポートは開いているが速度計測情報無し
    #[test]
    fn test_yp_open_host_with_nospeed() {
        let yp = YellowPageService::new(SiteConfig {
            yp_name: "yp".to_string(),
            listener_hideable: true,
            max_restrict_level: RestrictPortLevel::RestrictSpeed,
            restrict_speed: 2000,
        });

        {
            let mut host = checkd_host();
            host.upload_speed = None;

            let mut channel = ChannelMeta::Empty();
            channel.tracker_addr = Some("127.0.0.1:7144".parse().unwrap());
            channel.bitrate = 1000;
            channel.number_of_listener = 100;
            channel.number_of_relay = 100;
            channel.genre = "yp?ジャンル".to_string();

            let channels = yp.filter_channel_meta(&host, vec![channel.clone()]);
            assert_eq!(channels.len(), 1);
            assert_eq!(channels[0].tracker_addr.is_some(), true);
            assert_eq!(channels[0].display_genre.as_ref().unwrap(), "ジャンル");
            assert_eq!(channels[0].number_of_listener, -1);
            assert_eq!(channels[0].number_of_relay, -1);

            channel.genre = "yp@ジャンル".to_string();
            let channels = yp.filter_channel_meta(&host, vec![channel.clone()]);
            assert_eq!(channels[0].tracker_addr.is_some(), true);
            assert_eq!(channels[0].display_genre.as_ref().unwrap(), "ジャンル");
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_PORT_CHECK), false);
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_UPLOAD_NEEDS_TEST), false);
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_UPLOAD_NOT_EXCEED_LIMIT), false);
            assert_eq!(channels[0].number_of_listener, 100);
            assert_eq!(channels[0].number_of_relay, 100);

            channel.genre = "yp?@@ジャンル".to_string();
            let channels = yp.filter_channel_meta(&host, vec![channel.clone()]);
            assert_eq!(channels[0].tracker_addr.is_none(), true);
            assert_eq!(channels[0].display_genre.as_ref().unwrap(), "ジャンル");
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_PORT_CHECK), false);
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_UPLOAD_NEEDS_TEST), true);
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_UPLOAD_NOT_EXCEED_LIMIT), false);

            channel.genre = "yp?@@@ジャンル".to_string();
            let channels = yp.filter_channel_meta(&host, vec![channel.clone()]);
            assert_eq!(channels[0].tracker_addr.is_none(), true);
            assert_eq!(channels[0].display_genre.as_ref().unwrap(), "ジャンル");
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_PORT_CHECK), false);
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_UPLOAD_NEEDS_TEST), true);
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_UPLOAD_NOT_EXCEED_LIMIT), false);

            channel.genre = "yp?@@@@ジャンル".to_string();
            let channels = yp.filter_channel_meta(&host, vec![channel.clone()]);
            assert_eq!(channels[0].tracker_addr.is_none(), true);
            assert_eq!(channels[0].display_genre.as_ref().unwrap(), "ジャンル");
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_PORT_CHECK), false);
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_UPLOAD_NEEDS_TEST), true);
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_UPLOAD_NOT_EXCEED_LIMIT), false);
        }
    }

    /// ポートは開、速度計測情報あり、条件を下回る
    #[test]
    fn test_yp_open_host_with_speed_but_under() {
        let yp = YellowPageService::new(SiteConfig {
            yp_name: "yp".to_string(),
            listener_hideable: true,
            max_restrict_level: RestrictPortLevel::RestrictSpeed,
            restrict_speed: 2000,
        });
        {
            let mut host = checkd_host();
            host.upload_speed = Some(1000);

            let mut channel = ChannelMeta::Empty();
            channel.tracker_addr = Some("127.0.0.1:7144".parse().unwrap());
            channel.bitrate = 2000;
            channel.number_of_listener = 100;
            channel.number_of_relay = 100;
            channel.genre = "yp?@@ジャンル".to_string();

            let channels = yp.filter_channel_meta(&host, vec![channel.clone()]);
            assert_eq!(channels.len(), 1);
            assert_eq!(channels[0].tracker_addr.is_some(), false);
            assert_eq!(channels[0].display_genre.as_ref().unwrap(), "ジャンル");
            assert_eq!(channels[0].number_of_listener, -1);
            assert_eq!(channels[0].number_of_relay, -1);
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_PORT_CHECK), false);
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_UPLOAD_NEEDS_TEST), false);
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_UPLOAD_NOT_EXCEED_LIMIT), true);

            channel.genre = "yp@@@ジャンル".to_string();
            let channels = yp.filter_channel_meta(&host, vec![channel.clone()]);
            assert_eq!(channels.len(), 1);
            assert_eq!(channels[0].tracker_addr.is_some(), false);
            assert_eq!(channels[0].display_genre.as_ref().unwrap(), "ジャンル");
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_PORT_CHECK), false);
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_UPLOAD_NEEDS_TEST), false);
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_UPLOAD_NOT_EXCEED_LIMIT), true);

            channel.genre = "yp@@@@ジャンル".to_string();
            let channels = yp.filter_channel_meta(&host, vec![channel.clone()]);
            assert_eq!(channels.len(), 1);
            assert_eq!(channels[0].tracker_addr.is_some(), false);
            assert_eq!(channels[0].display_genre.as_ref().unwrap(), "ジャンル");
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_PORT_CHECK), false);
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_UPLOAD_NEEDS_TEST), false);
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_UPLOAD_NOT_EXCEED_LIMIT), true);
        }
    }

    /// ポートは開、速度計測情報あり、条件と等しい
    #[test]
    fn test_yp_open_host_with_speed_but_equals() {
        let yp = YellowPageService::new(SiteConfig {
            yp_name: "yp".to_string(),
            listener_hideable: true,
            max_restrict_level: RestrictPortLevel::RestrictSpeed,
            restrict_speed: 2000,
        });
        {
            let mut host = checkd_host();
            host.upload_speed = Some(2000);

            let mut channel = ChannelMeta::Empty();
            channel.tracker_addr = Some("127.0.0.1:7144".parse().unwrap());
            channel.bitrate = 2000;
            channel.number_of_listener = 100;
            channel.number_of_relay = 100;
            channel.genre = "yp?@@ジャンル".to_string();

            let channels = yp.filter_channel_meta(&host, vec![channel.clone()]);
            assert_eq!(channels.len(), 1);
            assert_eq!(channels[0].tracker_addr.is_some(), true);
            assert_eq!(channels[0].display_genre.as_ref().unwrap(), "ジャンル");
            assert_eq!(channels[0].number_of_listener, -1);
            assert_eq!(channels[0].number_of_relay, -1);
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_PORT_CHECK), false);
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_UPLOAD_NEEDS_TEST), false);
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_UPLOAD_NOT_EXCEED_LIMIT), false);

            channel.genre = "yp@@@ジャンル".to_string();
            let channels = yp.filter_channel_meta(&host, vec![channel.clone()]);
            assert_eq!(channels.len(), 1);
            assert_eq!(channels[0].tracker_addr.is_some(), true);
            assert_eq!(channels[0].display_genre.as_ref().unwrap(), "ジャンル");
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_PORT_CHECK), false);
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_UPLOAD_NEEDS_TEST), false);
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_UPLOAD_NOT_EXCEED_LIMIT), false);

            channel.genre = "yp@@@@ジャンル".to_string();
            let channels = yp.filter_channel_meta(&host, vec![channel.clone()]);
            assert_eq!(channels.len(), 1);
            assert_eq!(channels[0].tracker_addr.is_some(), true);
            assert_eq!(channels[0].display_genre.as_ref().unwrap(), "ジャンル");
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_PORT_CHECK), false);
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_UPLOAD_NEEDS_TEST), false);
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_UPLOAD_NOT_EXCEED_LIMIT), false);
        }
    }

    /// ポートは開、速度計測情報あり、条件を上回る
    #[test]
    fn test_yp_open_host_with_speed_but_over() {
        let yp = YellowPageService::new(SiteConfig {
            yp_name: "yp".to_string(),
            listener_hideable: true,
            max_restrict_level: RestrictPortLevel::RestrictSpeed,
            restrict_speed: 2000,
        });
        {
            let mut host = checkd_host();
            host.upload_speed = Some(3000);

            let mut channel = ChannelMeta::Empty();
            channel.tracker_addr = Some("127.0.0.1:7144".parse().unwrap());
            channel.bitrate = 2000;
            channel.number_of_listener = 100;
            channel.number_of_relay = 100;
            channel.genre = "yp?@@ジャンル".to_string();

            let channels = yp.filter_channel_meta(&host, vec![channel.clone()]);
            assert_eq!(channels.len(), 1);
            assert_eq!(channels[0].tracker_addr.is_some(), true);
            assert_eq!(channels[0].display_genre.as_ref().unwrap(), "ジャンル");
            assert_eq!(channels[0].number_of_listener, -1);
            assert_eq!(channels[0].number_of_relay, -1);
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_PORT_CHECK), false);
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_UPLOAD_NEEDS_TEST), false);
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_UPLOAD_NOT_EXCEED_LIMIT), false);

            channel.genre = "yp@@@ジャンル".to_string();
            let channels = yp.filter_channel_meta(&host, vec![channel.clone()]);
            assert_eq!(channels.len(), 1);
            assert_eq!(channels[0].tracker_addr.is_some(), true);
            assert_eq!(channels[0].display_genre.as_ref().unwrap(), "ジャンル");
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_PORT_CHECK), false);
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_UPLOAD_NEEDS_TEST), false);
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_UPLOAD_NOT_EXCEED_LIMIT), false);

            channel.genre = "yp@@@@ジャンル".to_string();
            let channels = yp.filter_channel_meta(&host, vec![channel.clone()]);
            assert_eq!(channels.len(), 1);
            assert_eq!(channels[0].tracker_addr.is_some(), true);
            assert_eq!(channels[0].display_genre.as_ref().unwrap(), "ジャンル");
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_PORT_CHECK), false);
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_UPLOAD_NEEDS_TEST), false);
            assert_eq!(channels[0].desc.contains(YellowPageService::MSG_UPLOAD_NOT_EXCEED_LIMIT), false);
        }
    }
}
