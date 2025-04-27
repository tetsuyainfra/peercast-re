

use chrono::{DateTime,  Utc};
use clap::{builder::TypedValueParser, Parser};
use libpeercast_re::pcp::GnuId;
use peercast_root::{FooterToml, IndexInfo};


fn main() {
    let args = Args::parse();
    // dbg!(&args);

    let info: IndexInfo = args.into();
    // dbg!(&info);

    let mut footer = FooterToml::default();
    footer.infomations = vec![info];

    let toml_str = toml::to_string_pretty(&footer).unwrap();
    println!("{}", toml_str);
}

const ABOUT: &str = r#"
peercast-root_footer.tomlを出力するためのプログラム
次のように使う
> $ create_info "名前1" > peercast_root_footer.toml
> $ create_info "名前2" >> peercast_root_footer.toml
> $ cat peercast_root_footer.toml
> [[infomations]]
> name = "名前1"
>
> [[infomations]]
> name = "名前2"
>
"#;

#[derive(Parser, Debug, Clone)]
#[command(name = env!("CARGO_BIN_NAME"))]
#[command(version, long_about = ABOUT)]
pub struct Args {
    /// 配信者名
    #[arg()]
    pub name: String,

    #[arg(long, default_value_t=GnuId::zero())]
    pub id : GnuId,

    #[arg(long="addr")]
    pub tracker_addr: Option<std::net::SocketAddr>,

    #[arg(long="url", default_value="")]
    pub contact_url: String,

    #[arg(long, default_value="")]
    pub genre: String,

    #[arg(long, default_value="")]
    pub desc: String,

    #[arg(long, default_value="")]
    pub comment: String,

    #[arg(long, default_value="")]
    pub stream_type: String,

    #[arg(long, default_value="")]
    pub stream_ext: String,

    #[arg(long, default_value_t=0)]
    pub bitrate: i32,

    #[arg(long, default_value_t=0)]
    pub number_of_listener: i32,

    #[arg(long, default_value_t=0)]
    pub number_of_relay: i32,

    #[arg(long, value_parser = clap::builder::StringValueParser::new().try_map(parse_datetime),)]
    pub created_at: Option<DateTime<Utc>>,
}


impl From<Args> for IndexInfo {
    fn from(v: Args) -> Self {
        let mut i = IndexInfo::default();
        i.id = v.id;
        i.name = v.name;
        i.tracker_addr = v.tracker_addr;
        i.contact_url = v.contact_url;
        i.genre = v.genre;
        i.desc = v.desc;
        i.comment = v.comment;
        i.stream_ext = v.stream_ext;
        i.bitrate = v.bitrate;
        i.number_of_listener = v.number_of_listener;
        i.number_of_relay = v.number_of_relay;
        i.created_at = v.created_at;

        i
    }
}


pub fn parse_datetime(value: String) -> Result<DateTime<Utc>, String> {
        use chrono::NaiveDate;

        if let Ok(datetime) = value.parse::<DateTime<Utc>>() {
            Ok(datetime)
        } else {
            let date = value
                .parse::<NaiveDate>()
                .map_err(|err| format!("valid RFC3339-formatted date or datetime: {err}"))?;
            Ok(date
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_local_timezone(Utc)
                .unwrap())
        }
}
