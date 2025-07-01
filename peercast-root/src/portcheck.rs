use std::{net::IpAddr, time::Duration};

use bb8_redis::RedisConnectionManager;
use redis::AsyncCommands;
use tokio::time::timeout;
use tracing::{debug, info};

use crate::{db::DatabaseConnection, REDIS_MASTER_KEY};

fn portcheck_key(host: IpAddr, port: u16) -> String {
    format!("{}:PORTCHECK:{}:{}", REDIS_MASTER_KEY(), host, port)
}

//-------------------------------------------------------------------------------
// PortCheck
//-------------------------------------------------------------------------------

#[repr(i8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortLevel {
    /// ポートチェックが行われていない
    None = 0,

    /// ポートチェックしたが疎通できなかった
    Incomplete = -1,

    /// 疎通OK
    Welldone = 1,

    /// 疎通OK, 速度OK
    WelldoneReachedUploadSpeed = 2,
}

pub async fn get_portcheck_level(
    DatabaseConnection(conn): &mut DatabaseConnection,
    host: IpAddr,
    port: u16,
) -> anyhow::Result<PortLevel> {
    info!(?host, ?port);
    let key = portcheck_key(host, port);
    // DBに結果を問い合わせ
    if let Some::<String>(port_level) = timeout(Duration::from_secs(1), conn.get(&key)).await?? {
        // あればそれを返す
        debug!(?port_level);
        Ok(PortLevel::Welldone)
    } else {
        // なければポートチェックする
        let port_level = portcheck(conn, host, port).await?;
        let _: Option<String> = timeout(Duration::from_secs(1), conn.set(&key, "true")).await??;
        Ok(port_level)
    }
}

pub async fn portcheck(
    conn: &mut bb8::PooledConnection<'static, RedisConnectionManager>,
    host: IpAddr,
    port: u16,
) -> anyhow::Result<PortLevel> {
    use libpeercast_re::pcp::{GnuId, PcpConnectionFactory};
    let self_addr = "0.0.0.0:7144".parse().unwrap();
    let factory = PcpConnectionFactory::builder(GnuId::new(), self_addr)
        .connect_timeout(Duration::from_secs(1))
        .build();

    let handshake = factory.connect((host, port).into()).await?;

    match handshake.ping().await {
        Ok(remote_id) => {
            Ok(PortLevel::Welldone) // ポートチェック成功
        }
        Err(_e) => {
            return Ok(PortLevel::Incomplete); // 失敗した場合は0を返す
        }
    }
}