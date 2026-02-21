use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};

use chrono::{DateTime, Utc};
use libpeercast_re::{
    ConnectionNo,
    pcp::connection2::{ConnectionFactory, SharedConnectionFactory},
};

use crate::{
    db::{CheckedHostRepository, SqliteCheckedHostRepository},
    model::{CheckedHost, PortLevel},
};

#[derive(thiserror::Error, Debug)]
pub enum HostCheckServiceError {
    #[error("再実行までの時間が経過していないため、ポートチェックをスキップします")]
    SkippedCheck,

    #[error("Database error")]
    InternalDB(#[from] sqlx::Error),

    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

pub struct HostCheckService;

impl HostCheckService {
    const CHECK_VALID_HOURS: Duration = Duration::from_hours(24);
    const CHECK_INTERVAL_SECS: Duration = Duration::from_secs(10);

    #[allow(dead_code)]
    /// ホストチェックの実装
    /// - `target_addr`: チェック対象のIPアドレス
    /// - `target_port`: チェック対象のポート番号
    /// - 戻り値: ポートの状態を示すPortLevel
    ///
    /// # Errors
    /// - `HostCheckServiceError::InternalDB`: データベース操作中にエラーが発生した場合
    /// - `HostCheckServiceError::Internal`: その他の内部エラーが発生した場合
    ////
    /// 内部の動作
    /// 1. データベースから対象ホストの情報を取得
    /// 2. その結果が以下の条件に基づいて処理を分岐
    ///    2.a ポート開放済みかつ有効期限内の場合は早期リターン
    ///    2.b 有効期限切れの場合はポートチェックを再実施
    ///    2.c ただし、ポートチェックの最終実行から一定時間経過していない場合は、再実施せずに早期リターン
    /// 3. ポートチェックは最終実行から一定時間経過している場合にのみ実施
    /// 4. ポートチェックの結果に基づいてデータベースを更新し、最終的なPortLevelを返す。
    pub async fn do_host_check(
        conn_factory: &SharedConnectionFactory,
        db_pool: &sqlx::Pool<sqlx::sqlite::Sqlite>,

        target_addr: IpAddr,
        target_port: u16,
        // ) -> Result<PortLevel, HostCheckServiceError> {
    ) -> Result<PortLevel, HostCheckServiceError> {
        let now = chrono::Utc::now();
        let checked_host_repo = SqliteCheckedHostRepository::new(db_pool.clone());
        let host = checked_host_repo.find_by_ip_port(target_addr, target_port).await?;

        if Self::is_port_opened(&host, now) {
            // ポート開放されているかつ有効期限内の場合は早期リターン
            return Ok(host.unwrap().port_level);
        }

        // 以降、ポートチェックは実施しないといけないが、最終実行から一定時間経過していない場合は、再実施せずに早期リターン
        if let Some(host) = &host {
            if now < host.updated_at + Self::CHECK_INTERVAL_SECS {
                // 最終実行から一定時間経過していない場合は、再実施せずに早期リターン
                return Err(HostCheckServiceError::SkippedCheck);
            }
        }

        // ポートチェックを実行
        Self::port_check(conn_factory, target_addr, target_port).await?;
        todo!()
    }

    /// ポートチェック
    /// - target_addr: チェック対象のIPアドレス
    /// - target_port: チェック対象のポート番号
    /// - 戻り値: (PortLevel, Option<u32>) ポートの状態と配信速度（速度が測定できない場合はNone）
    pub async fn port_check(
        conn_factory: &SharedConnectionFactory,
        target_addr: IpAddr,
        target_port: u16,
    ) -> anyhow::Result<(PortLevel, Option<u32>)> {
        let remote = SocketAddr::new(target_addr, target_port);
        let stream = tokio::net::TcpStream::connect(remote).await?;
        let _conn = conn_factory.create_outgoing_connection(ConnectionNo::new(), stream, remote);

        todo!()
    }

    // ポート開放されているか？
    fn is_port_opened(host: &Option<CheckedHost>, now: DateTime<Utc>) -> bool {
        let Some(host) = host else {
            // ホストが存在しない場合はFalseを返す
            return false;
        };

        // 現在時刻よりも、ホストの更新日時 + 有効期限の時間が小さい場合は、ホストの情報が古いとみなす
        if now > host.updated_at + Self::CHECK_VALID_HOURS {
            return false;
        }

        // ポートレベルがWelldone以上であれば、ポートが開放されているとみなす
        if host.port_level < PortLevel::Welldone {
            return false;
        }

        true
    }

    #[allow(dead_code)]
    fn is_recheck_ok(host: &CheckedHost, now: DateTime<Utc>) -> bool {
        // 現在時刻よりも、ホストの更新日時 + チェック間隔の時間が小さい場合は、再チェックできないとみなす
        if now < host.updated_at + Self::CHECK_INTERVAL_SECS {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use crate::model::DbIpAddr;

    use super::*;

    #[tokio::test]
    async fn test_do_host_check() {}

    #[allow(dead_code)]
    fn checked_host_mock() -> CheckedHost {
        CheckedHost {
            id: None,
            ip_address: DbIpAddr("0.0.0.0".parse().unwrap()),
            port: 0,
            port_level: PortLevel::None,
            port_speed: None,
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_return_or_check() {
        let now = Utc::now();

        // ホストが存在しない場合はFalseを返す
        assert_eq!(HostCheckService::is_port_opened(&None, now), false);

        // let now = Utc::now();
        // // 有効期限切れの時間
        // let expried = now - HostCheckService::CHECK_VALID_HOURS - Duration::from_secs(1);

        // // 再チェックまでの時間
        // let check_interval = now - HostCheckService::CHECK_INTERVAL_SECS - Duration::from_secs(1);

        // // (port open, not expired) => True
        // // (true, true)
        // // ホストが存在し、有効期限切れでポート開放されている場合はTrueを返す
        // let mut host = checked_host_mock();
        // host.port_level = PortLevel::Welldone;
        // host.port_speed = Some(1000);
        // host.updated_at = now;
        // assert_eq!(HostCheckService::is_return_or_check(&Some(host), now), true);

        // // (true, false)
        // // ホストが存在し、有効期限切れだけどポート開放されている場合はTrueを返す
        // let mut host = checked_host_mock();
        // host.port_level = PortLevel::Welldone;
        // host.port_speed = Some(1000);
        // host.updated_at = now;
        // host.updated_at = now - HostCheckService::CHECK_VALID_HOURS - Duration::from_secs(1);
        // assert_eq!(HostCheckService::is_return_or_check(&Some(host), now), false);

        // // (true, false)
        // // ホストが存在し、有効期限内でポート開放されていない場合はFalseを返す
        // let mut host = checked_host_mock();
        // host.port_level = PortLevel::Incomplete;
        // host.port_speed = None;
        // host.updated_at = now;
        // host.updated_at = now - HostCheckService::CHECK_VALID_HOURS / 2;
        // assert_eq!(HostCheckService::is_return_or_check(&Some(host), now), false);
    }
}
