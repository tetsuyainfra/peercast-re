use crate::pcp::{Atom, ChildAtom, Id4, ParentAtom};

pub struct RootBuilder {
    update_interval: Option<u32>,
    next_update_interval: Option<u32>,
    download_path: Option<String>,
    check_version: Option<u32>,
    pcp_msg_ascii: Option<String>,
    is_set_root_update: bool,
}

impl Default for RootBuilder {
    fn default() -> Self {
        Self {
            update_interval: Some(30),      // 30 sec
            next_update_interval: Some(30), // 30 sec
            check_version: Some(crate::PKG_SERVANT_VERSION),
            download_path: Some("donwload.php".into()),
            pcp_msg_ascii: Some("".into()),
            is_set_root_update: false,
        }
    }
}

impl RootBuilder {
    //
    pub fn new() -> Self {
        Self {
            update_interval: None,
            next_update_interval: None,
            download_path: None,
            check_version: None,
            pcp_msg_ascii: None,
            is_set_root_update: false,
        }
    }

    /// update_interval : 情報更新の時間間隔(sec)
    pub fn set_update_interval(mut self, update_interval: u32) -> Self {
        self.update_interval = Some(update_interval);
        self
    }

    /// next_update_interval : 次の情報更新までの時間(sec)
    pub fn set_next_update_interval(mut self, next_update_interval: u32) -> Self {
        self.next_update_interval = Some(next_update_interval);
        self
    }

    pub fn set_download_path(mut self, str: String) -> Self {
        self.download_path = Some(str);
        self
    }

    pub fn set_msg(mut self, ascii_string: String) -> Self {
        self.pcp_msg_ascii = Some(ascii_string);
        self
    }

    /// flag_root_update: PCP_BCSTでChannelInfo情報の更新を促す
    pub fn set_root_update(mut self, flag: bool) -> Self {
        self.is_set_root_update = flag;
        self
    }

    pub fn build(self) -> Atom {
        let mut atoms: Vec<Atom> = Vec::with_capacity(6);
        if self.update_interval.is_some() {
            atoms
                .push(ChildAtom::from((Id4::PCP_ROOT_UPDINT, self.update_interval.unwrap())).into())
        }
        if self.download_path.is_some() {
            atoms.push(ChildAtom::from((Id4::PCP_ROOT_URL, self.download_path.unwrap())).into());
        }
        if self.check_version.is_some() {
            atoms.push(
                ChildAtom::from((Id4::PCP_ROOT_CHECKVER, self.check_version.unwrap())).into(),
            );
        }
        if self.next_update_interval.is_some() {
            atoms.push(
                ChildAtom::from((Id4::PCP_ROOT_NEXT, self.next_update_interval.unwrap())).into(),
            )
        }
        if self.pcp_msg_ascii.is_some() {
            atoms.push(ChildAtom::from((Id4::PCP_MESG_ASCII, self.pcp_msg_ascii.unwrap())).into())
        }
        if self.is_set_root_update {
            atoms.push(ParentAtom::from((Id4::PCP_ROOT_UPDATE, vec![])).into())
        }

        ParentAtom::from((Id4::PCP_ROOT, atoms)).into()
    }
}

// Updateを促すATOMを作成する
impl RootBuilder {
    pub fn build_update_request() -> Atom {
        Self::new().set_root_update(true).build()
    }
}
