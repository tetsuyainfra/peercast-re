use std::net::IpAddr;

use clap::Parser;
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use url::Url;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    println!("DB_URI: {}", args.database_url.as_str());

    let pool = SqlitePoolOptions::new().connect(args.database_url.as_str()).await?;
    let migrate = sqlx::migrate!("./migrations").run(&pool).await?;
    println!("MIGRATED: {:?}", migrate);
    // let hosts: Vec<_> = sqlx::query_as("select id, name, created_at from checked_host").fetch_all(&pool).await?;
    let hosts: Vec<_> = sqlx::query_as!(CheckedHost, "select id, name from checked_hosts").fetch_all(&pool).await?;
    dbg!(hosts);

    match args.command {
        SubCommand::List => {}
        SubCommand::Show {
            key,
        } => todo!(),
    }

    Ok(())
}
////////////////////////////////////////////////////////////////////////////////
//  List
//

////////////////////////////////////////////////////////////////////////////////
//  Repository
//

#[derive(Debug, sqlx::FromRow)]
struct CheckedHost {
    pub id: i64,
    pub name: String, // ip: IpAddr,
                      // port: u16,
                      // stats: bool
}

struct CheckedHostRepository {
    pool: SqlitePool,
}

impl CheckedHostRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
        }
    }

    pub async fn all(&self) {}
}

////////////////////////////////////////////////////////////////////////////////
//  CLI
//
const ABOUT: &str = r#"
peercast-rootのKVSを参照するためのプログラム
次のように使う
> $ root-kvs list
> $ root-kvs show <key>
"#;

#[derive(Debug, Clone, Parser)]
#[command(name = env!("CARGO_BIN_NAME"))]
#[command(version, long_about = ABOUT)]
pub struct Args {
    #[arg(short, long, env, value_parser = clap::builder::ValueParser::new(Url::parse), default_value="sqlite::memory:")]
    pub database_url: Url,

    #[command(subcommand)]
    pub command: SubCommand,
}

#[derive(Debug, Clone, Parser)]
pub enum SubCommand {
    /// KVSの内容を一覧表示する
    List,
    /// 指定したkeyの内容を表示する
    Show {
        /// 取得するkey
        key: String,
    },
}
