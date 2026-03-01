use axum::Json;
use peercast_root::{IndexInfo, model::JsonChannelInfo};

#[derive(Debug)]
pub struct YellowPage {
    #[allow(unused)]
    yp_name_space: String,
    footer_channels: Vec<IndexInfo>,
}

impl YellowPage {
    pub fn new(yp_name_space: &str) -> Self {
        Self {
            yp_name_space: yp_name_space.to_string(),
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
    pub fn to_index_json(&self, config: &FilterConfig, channels: Vec<JsonChannelInfo>) -> Json<Vec<JsonChannelInfo>> {
        let filtered_channels = filter_channels(config, channels);
        let channels = merge_footer(&self.footer_channels, filtered_channels);

        Json(channels)
    }

    pub fn to_index_txt(&self, config: &FilterConfig, channels: Vec<JsonChannelInfo>) -> Vec<String> {
        let filtered_channels = filter_channels(config, channels);
        let channels = merge_footer(&self.footer_channels, filtered_channels);

        channels.into_iter().map(|c| c.to_line_of_index_txt()).collect()
    }
}

fn filter_channels(_config: &FilterConfig, channels: Vec<JsonChannelInfo>) -> Vec<JsonChannelInfo> {
    // TODO: implement filtering logic
    channels
}

fn merge_footer(footer_channels: &Vec<IndexInfo>, mut channels: Vec<JsonChannelInfo>) -> Vec<JsonChannelInfo> {
    channels.reserve(footer_channels.len());
    channels.extend(footer_channels.iter().map(|e| e.into()));
    channels
}

pub struct FilterConfig {}
