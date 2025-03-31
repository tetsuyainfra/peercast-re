use crate::pcp::{Atom, ChildAtom, Id4, ParentAtom, TrackInfo};

pub struct TrackInfoBuilder {
    pub track: TrackInfo,
}

impl TrackInfoBuilder {
    pub fn new(track: TrackInfo) -> TrackInfoBuilder {
        Self { track }
    }
    pub fn build(self) -> Atom {
        let TrackInfo {
            title,
            creator,
            url,
            album,
            genre,
        } = self.track;
        ParentAtom::from((
            Id4::PCP_CHAN_TRACK,
            vec![
                ChildAtom::from((Id4::PCP_CHAN_TRACK_TITLE, title)).into(),
                ChildAtom::from((Id4::PCP_CHAN_TRACK_CREATOR, creator)).into(),
                ChildAtom::from((Id4::PCP_CHAN_TRACK_URL, url)).into(),
                ChildAtom::from((Id4::PCP_CHAN_TRACK_ALBUM, album)).into(),
                ChildAtom::from((Id4::PCP_CHAN_TRACK_GENRE, genre)).into(),
            ],
        ))
        .into()
    }
}
