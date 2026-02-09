use peercast_root::{IndexInfo, channel::json_model::JsonChannel};

#[derive(Debug)]
pub struct YellowPage {
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
    pub fn to_index_json(&self, mut channels: Vec<JsonChannel>) -> Vec<JsonChannel> {
        channels.reserve(self.footer_channels.len());
        channels.extend(self.footer_channels.iter().map(|e| e.into()));

        Vec::new()
    }

    pub fn to_index_txt(&self, channels: Vec<JsonChannel>) -> Vec<String> {
        self.to_index_json(channels).into_iter().map(|c| c.to_line_of_index_txt()).collect()
    }
}
