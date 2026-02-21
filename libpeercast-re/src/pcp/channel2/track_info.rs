use crate::pcp::builder2::TrackInfo;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ValidTrackInfo {
    pub title: String,
    pub creator: String,
    pub url: String,
    pub album: String,
    pub genre: String,
}

impl From<&TrackInfo> for ValidTrackInfo {
    fn from(info: &TrackInfo) -> Self {
        let TrackInfo {
            title,
            creator,
            url,
            album,
            genre,
        } = info;

        let mut vti = Self::default();
        if let Some(title) = title {
            vti.title = String::from_utf8_lossy(title).to_string();
        }
        if let Some(creator) = creator {
            vti.creator = String::from_utf8_lossy(creator).to_string();
        }
        if let Some(url) = url {
            vti.url = String::from_utf8_lossy(url).to_string();
        }
        if let Some(album) = album {
            vti.album = String::from_utf8_lossy(album).to_string();
        }
        if let Some(genre) = genre {
            vti.genre = String::from_utf8_lossy(genre).to_string();
        }

        vti
    }
}
