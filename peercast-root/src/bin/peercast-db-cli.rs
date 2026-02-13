use std::net::IpAddr;

use clap::Parser;
use peercast_root::db::{CheckedHostRepository, checked_hosts::SqliteCheckedHostRepository};
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
                println!(
                    "ID: {}, IP: {}, port: {}, speed: {}, created_at: {:?}",
                    host.id.unwrap(),
                    host.ip_address.0,
                    host.port,
                    host.speed,
                    host.updated_at
                );
            });
        }
        SubCommand::Find {
            ip,
            port,
        } => {
            let hosts = match port {
                None => repo.find_all_by_ip(ip).await?,
                Some(port) => {
                    let host = repo.find_by_ip_port(ip, port).await?;
                    host.into_iter().collect()
                }
            };
            if hosts.is_empty() {
                println!("Host with IP {} not found.", ip);
            } else {
                for host in hosts {
                    println!(
                        "ID: {}, IP: {}, port: {}, speed: {}, created_at: {:?}",
                        host.id.unwrap(),
                        host.ip_address.0,
                        host.port,
                        host.speed,
                        host.updated_at
                    );
                }
            }
        }
        SubCommand::Show {
            id,
        } => {
            let host = repo.find_by_id(id).await?;
            if let Some(host) = host {
                println!(
                    "ID: {}, IP: {}, port: {}, speed: {}, updated_at: {:?}",
                    host.id.unwrap(),
                    host.ip_address.0,
                    host.port,
                    host.speed,
                    host.updated_at
                );
            } else {
                println!("Host with ID {} not found.", id);
            }
        }
        SubCommand::Add {
            ip,
            port,
            speed,
        } => {
            let id = repo.insert(ip, port, speed).await?;
            let host = repo.find_by_id(id).await?;
            if let Some(h) = host {
                println!(
                    "Added host with IP: {}, port: {}, speed: {}, updated_at: {:?}",
                    *h.ip_address, h.port, h.speed, h.updated_at
                );
            } else {
                println!("Failed to add host with IP: {}, port: {}", ip, port);
            }
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

    /// IPからチェック済みホストを検索する
    Find {
        /// 取得するIPアドレス
        ip: IpAddr,
        port: Option<u16>,
    },

    /// 指定したIDの内容を表示する
    Show {
        /// 取得するID
        id: i64,
    },

    /// チェック済みホストを追加する
    Add {
        ip: IpAddr,
        port: u16,
        speed: i32,
    },
}
