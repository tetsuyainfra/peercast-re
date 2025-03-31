/// Id4はネットワークから送られて来たバイトストリーム4バイトをそのままの並びをu32へ格納したものである。
/// これはでバイナリの表現形式でいうBigEndianに該当する
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Id4(pub u32);

impl Id4 {
    // 定数の定義はファイル下部にあり
}

// Conversion: * -> Id4
impl From<u32> for Id4 {
    fn from(b: u32) -> Self {
        Id4(b)
    }
}

impl From<[u8; 4]> for Id4 {
    fn from(b: [u8; 4]) -> Self {
        Id4(u32::from_be_bytes(b))
    }
}

// Conversion: Id4 -> *
// これ無い方がいいのでは？
// impl From<Id4> for u32 {
//     fn from(value: Id4) -> Self {
//         value.0
//     }
// }

impl From<Id4> for [u8; 4] {
    fn from(value: Id4) -> Self {
        value.0.to_be_bytes()
    }
}

impl From<Id4> for u32 {
    fn from(value: Id4) -> Self {
        value.0
    }
}

impl std::fmt::Debug for Id4 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Id4(")?;
        internal::fmt(self.0, f)?;
        write!(f, ")")?;
        Ok(())
    }
}

mod internal {
    pub(super) fn fmt(val: u32, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // write!(f, "b\"")?;
        for b in val.to_be_bytes() {
            // https://doc.rust-lang.org/reference/tokens.html#byte-escapes
            if b == b'\n' {
                write!(f, "\\n")?;
            } else if b == b'\r' {
                write!(f, "\\r")?;
            } else if b == b'\t' {
                write!(f, "\\t")?;
            } else if b == b'\\' || b == b'"' {
                write!(f, "\\{}", b as char)?;
            } else if b == b'\0' {
                write!(f, "\\0")?;
            // ASCII printable
            } else if (0x20..0x7f).contains(&b) {
                write!(f, "{}", b as char)?;
            } else {
                write!(f, "\\x{:02x}", b)?;
            }
        }
        // write!(f, "\"")?;
        Ok(())
    }
}

// 定数の定義
// #[allow(non_snake_case)]
// #[allow(non_upper_case_globals)]
macro_rules! def_id4 {
  ($($KEY:ident = $VAL:expr),*,) => {
      impl Id4 {
        $(
          pub const $KEY: Id4 = Id4(u32::from_be_bytes(*$VAL));
        )*
      }
  };
}
def_id4! {
  PCP_CONNECT        = b"pcp\n",

  PCP_OK             = b"ok\0\0",

  PCP_HELO           = b"helo",
  PCP_HELO_AGENT     = b"agnt",
  PCP_HELO_OSTYPE    = b"ostp",
  PCP_HELO_SESSIONID = b"sid\0",
  PCP_HELO_PORT      = b"port",
  PCP_HELO_PING      = b"ping",
  PCP_HELO_PONG      = b"pong",
  PCP_HELO_REMOTEIP  = b"rip\0",
  PCP_HELO_VERSION   = b"ver\0",
  PCP_HELO_BCID      = b"bcid",
  PCP_HELO_DISABLE   = b"dis\0", // BANされている時に通知する

  PCP_OLEH           = b"oleh",

  PCP_MODE           = b"mode",
  PCP_MODE_GNUT06    = b"gn06",

  PCP_ROOT           = b"root",
  PCP_ROOT_UPDINT    = b"uint",
  PCP_ROOT_CHECKVER  = b"chkv",
  PCP_ROOT_URL       = b"url\0",
  PCP_ROOT_UPDATE    = b"upd\0",
  PCP_ROOT_NEXT      = b"next",

  PCP_OS_LINUX       = b"lnux",
  PCP_OS_WINDOWS     = b"w32\0",
  PCP_OS_OSX         = b"osx\0",
  PCP_OS_WINAMP      = b"wamp",
  PCP_OS_ZAURUS      = b"zaur",

  PCP_GET            = b"get\0",
  PCP_GET_ID         = b"id\0\0",
  PCP_GET_NAME       = b"name",

  PCP_HOST           = b"host",
  PCP_HOST_ID        = b"id\0\0",
  PCP_HOST_BCID      = b"bcid", // PeerCast Stationだけ？
  PCP_HOST_IP        = b"ip\0\0",
  PCP_HOST_PORT      = b"port",
  PCP_HOST_NUML      = b"numl",
  PCP_HOST_NUMR      = b"numr",
  PCP_HOST_UPTIME    = b"uptm",
  PCP_HOST_TRACKER   = b"trkr",
  PCP_HOST_CHANID    = b"cid\0",
  PCP_HOST_VERSION   = b"ver\0",
  PCP_HOST_VERSION_VP = b"vevp",
  PCP_HOST_VERSION_EX_PREFIX = b"vexp",
  PCP_HOST_VERSION_EX_NUMBER = b"vexn",
  PCP_HOST_FLAGS1    = b"flg1",
  PCP_HOST_OLDPOS    = b"oldp",
  PCP_HOST_NEWPOS    = b"newp",
  PCP_HOST_UPHOST_IP = b"upip",
  PCP_HOST_UPHOST_PORT = b"uppt",
  PCP_HOST_UPHOST_HOPS = b"uphp",

  PCP_QUIT           = b"quit",

  PCP_CHAN           = b"chan",
  PCP_CHAN_ID        = b"id\0\0",
  PCP_CHAN_BCID      = b"bcid",
  PCP_CHAN_KEY       = b"key\0",

  PCP_CHAN_PKT       = b"pkt\0",
  PCP_CHAN_PKT_TYPE  = b"type",
  PCP_CHAN_PKT_POS   = b"pos\0",
  PCP_CHAN_PKT_HEAD  = b"head",
  PCP_CHAN_PKT_DATA  = b"data",
  PCP_CHAN_PKT_META  = b"meta",
  PCP_CHAN_PKT_CONTINUATION = b"cont",

  PCP_CHAN_INFO          = b"info",
  PCP_CHAN_INFO_TYPE     = b"type",
  PCP_CHAN_INFO_STREAMTYPE       = b"styp",
  PCP_CHAN_INFO_STREAMEXT        = b"sext",
  PCP_CHAN_INFO_BITRATE  = b"bitr",
  PCP_CHAN_INFO_GENRE    = b"gnre",
  PCP_CHAN_INFO_NAME     = b"name",
  PCP_CHAN_INFO_URL      = b"url\0",
  PCP_CHAN_INFO_DESC     = b"desc",
  PCP_CHAN_INFO_COMMENT  = b"cmnt",

  PCP_CHAN_TRACK         = b"trck",
  PCP_CHAN_TRACK_TITLE   = b"titl",
  PCP_CHAN_TRACK_CREATOR = b"crea",
  PCP_CHAN_TRACK_URL     = b"url\0",
  PCP_CHAN_TRACK_ALBUM   = b"albm",
  PCP_CHAN_TRACK_GENRE   = b"gnre",

  PCP_MESG               = b"mesg",
  PCP_MESG_ASCII         = b"asci",       // ascii/sjis to be depreciated.. utf8/unicode is the only supported format from now.
  PCP_MESG_SJIS          = b"sjis",

  PCP_BCST               = b"bcst",
  PCP_BCST_TTL           = b"ttl\0",
  PCP_BCST_HOPS          = b"hops",
  PCP_BCST_FROM          = b"from",
  PCP_BCST_DEST          = b"dest",
  PCP_BCST_GROUP         = b"grp\0",
  PCP_BCST_CHANID        = b"cid\0",
  PCP_BCST_VERSION       = b"vers",
  PCP_BCST_VERSION_VP = b"vrvp",
  PCP_BCST_VERSION_EX_PREFIX = b"vexp",
  PCP_BCST_VERSION_EX_NUMBER = b"vexn",

  PCP_PUSH               = b"push",
  PCP_PUSH_IP            = b"ip\0\0",
  PCP_PUSH_PORT          = b"port",
  PCP_PUSH_CHANID        = b"cid\0",

  PCP_SPKT               = b"spkt",

  PCP_ATOM               = b"atom",

  PCP_SESSIONID          = b"sid\0",
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id4() {
        assert_eq!(Id4::PCP_OK.0, u32::from_be_bytes(*b"ok\0\0"));
        let b: [u8; 4] = Id4::PCP_OK.clone().into();
        assert_eq!(b[0], b'o');
        assert_eq!(b[1], b'k');
        assert_eq!(b[2], b'\0');
        assert_eq!(b[3], b'\0');

        let s = format!("{:?}", Id4::PCP_OK);
        // assert_eq!(s, r#"Id4(b"ok\0\0")"#); // ちょっと冗長
        assert_eq!(s, r#"Id4(ok\0\0)"#);
    }
}
