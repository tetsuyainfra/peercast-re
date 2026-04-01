use crate::pcp::{AtomMut, Id4};

pub struct RootBuilder2 {
    update_interval: Option<u32>,
    next_update_interval: Option<u32>,
    download_path: Option<String>,
    check_version: Option<u32>,
    pcp_msg_ascii: Option<String>,
    is_set_root_update: bool,
}

impl Default for RootBuilder2 {
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

impl RootBuilder2 {
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

    pub fn build(self) -> AtomMut {
        let mut atoms: Vec<AtomMut> = Vec::with_capacity(6);
        if let Some(v) = self.update_interval {
            atoms.push((Id4::PCP_ROOT_UPDINT, v).into());
        }
        if let Some(v) = self.next_update_interval {
            atoms.push((Id4::PCP_ROOT_NEXT, v).into());
        }
        if let Some(v) = self.download_path {
            atoms.push((Id4::PCP_ROOT_URL, v).into());
        }
        if let Some(v) = self.check_version {
            atoms.push((Id4::PCP_ROOT_CHECKVER, v).into());
        }
        if let Some(v) = self.next_update_interval {
            atoms.push((Id4::PCP_ROOT_NEXT, v).into());
        }
        if let Some(v) = self.pcp_msg_ascii {
            atoms.push((Id4::PCP_MESG_ASCII, v).into());
        }

        if self.is_set_root_update {
            atoms.push((Id4::PCP_ROOT_UPDATE, Vec::<AtomMut>::new()).into());
        }

        AtomMut::from((Id4::PCP_ROOT, atoms))
    }
}

// Updateを促すATOMを作成する
impl RootBuilder2 {
    pub fn build_update_request() -> AtomMut {
        Self::new().set_root_update(true).build()
    }
}
