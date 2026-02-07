use std::net::IpAddr;

use axum_extra::headers::Date;
use chrono::DateTime;
use clap::Parser;
use peercast_root::db::{CheckedHost, CheckedHostRepository, DbIpAddr, checked_host::SqliteCheckedHostRepository};
use sqlx::sqlite::SqlitePoolOptions;
use url::Url;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    println!("DB_URI: {}", args.database_url.as_str());

    let pool = SqlitePoolOptions::new().connect(args.database_url.as_str()).await?;
    let repo = SqliteCheckedHostRepository::new(pool.clone());

    match args.command {
        SubCommand::List => {
            repo.all().await?.iter().for_each(|host| {
                println!("ID: {}, IP: {} created_at: {:?}", host.id.unwrap(), host.ip.0, host.created_at);
            });
        }
        SubCommand::Show {
            id,
        } => {
            let host = repo.find_by_id(id).await?;
            if let Some(host) = host {
                println!("ID: {}, IP: {} created_at: {:?}", host.id.unwrap(), host.ip.0, host.created_at);
            } else {
                println!("Host with ID {} not found.", id);
            }
        }
        SubCommand::Add {ip } => {
            let new_val = CheckedHost {
                id: None,
                ip: DbIpAddr(ip),
                created_at: DateTime::default(),
            };
            repo.add(&new_val).await?;
            let host = repo.find_by_ip(&new_val.ip).await?.unwrap();
            println!("Added host with IP: {}, created_at: {:?}", *host.ip, host.created_at);
        }
    }

    Ok(())
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
        id: i64,
    },
    Add {
        ip: IpAddr
    },
}
