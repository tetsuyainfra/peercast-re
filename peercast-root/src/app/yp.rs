use nom::Check;
use peercast_root::{
    RestrictPortLevel,
    model::{ChannelMeta, CheckedHost, IndexInfo, PortLevel},
};
use regex::Regex;

#[derive(Debug)]
pub struct YellowPage {
    config: SiteConfig,
    footer_channels: Vec<IndexInfo>,
}

#[derive(Debug)]
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

impl YellowPage {
    pub fn new(site_config: SiteConfig) -> Self {
        Self {
            config: site_config,
            footer_channels: Vec::new(),
        }
    }
    pub fn add_footer_channels(mut self, index_infos: Vec<IndexInfo>) -> Self {
        self.footer_channels.reserve(index_infos.len());
        self.footer_channels.extend(index_infos.into_iter());
        self
    }
}

impl YellowPage {
    pub fn filter_channel_meta(&self, host: CheckedHost, channels: Vec<ChannelMeta>) -> Vec<ChannelMeta> {
        let filtered_channels = Self::filter_channels(&self.config, host, channels);
        let channels = Self::merge_footer(&self.footer_channels, filtered_channels);

        channels
    }

    fn filter_channels(config: &SiteConfig, host: CheckedHost, channels: Vec<ChannelMeta>) -> Vec<ChannelMeta> {
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
            .filter_map(|ch| {
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

                // @数 = Level
                let at_count = rest.chars().take_while(|c| *c == '@').count();
                rest = &rest[at_count..];

                // ---- 制限チェック ----

                /*
                match at_count {
                    0 => {}

                    // @ → ポートチェック
                    1 => {
                        if host.port_level != PortLevel::Open {
                            return None;
                        }
                    }

                    // @@ → ポート + 帯域(ビットレート)
                    2 => {
                        if host.port_level != PortLevel::Open {
                            return None;
                        }
                        if let Some(speed) = net.port_speed {
                            if speed < ch.bitrate {
                                return None;
                            }
                        } else {
                            return None;
                        }
                    }

                    // @@@ → ポート + 2Mbps制限
                    3 => {
                        if user.port_level != PortLevel::Open {
                            return None;
                        }
                        if let Some(speed) = user.port_speed {
                            if speed < 2_000_000 {
                                return None;
                            }
                        } else {
                            return None;
                        }
                    }

                    _ => {}
                }
                */

                return Some(ch);
            })
            .collect();

        channels
    }

    fn merge_footer(footer_channels: &Vec<IndexInfo>, mut channels: Vec<ChannelMeta>) -> Vec<ChannelMeta> {
        channels.reserve(footer_channels.len());
        channels.extend(footer_channels.iter().map(|e| e.into()));
        channels
    }
}

struct SplitGenre<'a> {
    namespace: &'a str,
    hide: bool,
    at_mark: u8,
    genre: &'a str,
}

#[cfg(test)]
mod test {
    use peercast_root::model::{ChannelMeta, CheckedHost};

    use crate::app::yp::{SiteConfig, YellowPage};
    #[test]
    fn test_re() {
        let pattern = format!(r"^{}(\??)(@{{0,3}})(.+)", "yp");
        println!("pattern: {}", &pattern);
        let re = regex::Regex::new(&pattern).unwrap();

        let tgt = "ypGenreジャンル";
        println!("tgt: {}", tgt);
        for m in re.find(tgt).iter() {
            dbg!(m);
        }

        let tgt = "yp?Genreジャンル";
        println!("tgt: {}", tgt);
        for m in re.find(tgt).iter() {
            dbg!(m);
        }
    }

    #[test]
    fn test_yp() {
        let yp = YellowPage::new(SiteConfig {
            yp_name: "yp".to_string(),
            listener_hideable: true,
            max_restrict_level: peercast_root::RestrictPortLevel::PortCheck,
            restrict_speed: 2000,
        });

        {
            let host = CheckedHost {
                id: todo!(),
                ip_address: todo!(),
                port: todo!(),
                port_level: todo!(),
                upload_speed: todo!(),
                updated_at: todo!(),
            };
            let mut channel = ChannelMeta::Empty();
            channel.number_of_listener = 100;
            channel.number_of_relay = 100;
            channel.genre = "ypXYZ?genre".to_string();

            let channels = yp.filter_channel_meta(host, vec![channel]);
            assert_eq!(channels.len(), 1);
            assert_eq!(channels[0].display_genre.as_ref().unwrap(), "genre");
            assert_eq!(channels[0].number_of_listener, -1);
            assert_eq!(channels[0].number_of_relay, -1);
        }
    }
}
