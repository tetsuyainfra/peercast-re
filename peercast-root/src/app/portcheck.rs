use std::net::IpAddr;

use peercast_root::PortLevel;
// use redis::AsyncCommands;
// use tokio::time::timeout;

// fn portcheck_key(redis_master_key: &str, host: IpAddr, port: u16) -> String {
//     format!("{}:PORTCHECK:{}:{}", redis_master_key, host, port)
// }

//-------------------------------------------------------------------------------
// PortCheck
//-------------------------------------------------------------------------------
#[allow(dead_code)]
pub async fn get_portcheck_level(
    _redis_master_key: &str,
    // DatabaseConnection(conn): &mut DatabaseConnection,
    _host: IpAddr,
    _port: u16,
) -> anyhow::Result<PortLevel> {
    // let key = portcheck_key(redis_master_key, host, port);
    // // DBに結果を問い合わせ
    // if let Some::<String>(port_level) = timeout(Duration::from_secs(1), conn.get(&key)).await?? {
    //     // あればそれを返す
    //     debug!(?port_level);
    //     Ok(PortLevel::Welldone)
    // } else {
    //     // なければポートチェックする
    //     let port_level = match portcheck(conn, host, port).await? {
    //         true => {
    //             debug!("Port check succeeded for {}:{}", host, port);
    //             PortLevel::Welldone
    //         }
    //         false => {
    //             debug!("Port check failed for {}:{}", host, port);
    //             PortLevel::Incomplete
    //         }
    //     };

    //     // ポートチェック結果をDBに保存
    //     let _: Option<String> = timeout(Duration::from_secs(1), conn.set(&key, "true")).await??;
    //     Ok(port_level)
    // }
    Ok(PortLevel::None)
}

// pub async fn portcheck(
//     _conn: &mut bb8::PooledConnection<'static, RedisConnectionManager>,
//     host: IpAddr,
//     port: u16,
// ) -> anyhow::Result<bool> {
//     use libpeercast_re::pcp::{GnuId, PcpConnectionFactory};
//     let self_addr = "0.0.0.0:7144".parse().unwrap();
//     let factory =
//         PcpConnectionFactory::builder(GnuId::new(), self_addr).connect_timeout(Duration::from_secs(1)).build();

//     let handshake = match factory.connect((host, port).into()).await {
//         Ok(h) => h,
//         Err(_e) => {
//             info!("Failed to connect to {}:{}", host, port);
//             return Ok(false);
//         }
//     };

//     match handshake.ping().await {
//         Ok(remote_id) => {
//             info!("Success to PCP connect to {}:{}({})", host, port, remote_id);
//             // HACKME: handshakeの切断処理を確認する
//             return Ok(true);
//         }
//         Err(_e) => {
//             info!("Failed to PCP connect to {}:{}", host, port);
//             return Ok(false);
//         }
//     }
// }
