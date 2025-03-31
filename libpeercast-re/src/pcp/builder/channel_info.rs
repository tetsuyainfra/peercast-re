use crate::pcp::{Atom, ChannelInfo, ChildAtom, Id4, ParentAtom};

pub struct ChannelInfoBuilder {
    pub info: ChannelInfo,
}

impl ChannelInfoBuilder {
    pub fn new(info: ChannelInfo) -> ChannelInfoBuilder {
        Self { info }
    }
    pub fn build(self) -> Atom {
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
        } = self.info;

        ParentAtom::from((
            Id4::PCP_CHAN_INFO,
            vec![
                ChildAtom::from((Id4::PCP_CHAN_INFO_NAME, name)).into(),
                ChildAtom::from((Id4::PCP_CHAN_INFO_TYPE, typ)).into(),
                ChildAtom::from((Id4::PCP_CHAN_INFO_GENRE, genre)).into(),
                ChildAtom::from((Id4::PCP_CHAN_INFO_DESC, desc)).into(),
                ChildAtom::from((Id4::PCP_CHAN_INFO_COMMENT, comment)).into(),
                ChildAtom::from((Id4::PCP_CHAN_INFO_URL, url)).into(),
                ChildAtom::from((Id4::PCP_CHAN_INFO_STREAMTYPE, stream_type)).into(),
                ChildAtom::from((Id4::PCP_CHAN_INFO_STREAMEXT, stream_ext)).into(),
                ChildAtom::from((Id4::PCP_CHAN_INFO_BITRATE, bitrate)).into(),
            ],
        ))
        .into()
    }
}
