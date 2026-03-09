use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use libpeercast_re::pcp::{
    GnuId,
    connection5::{ConnectionFactory, OutgoingConnection, ping::Ping},
};

use crate::{
    connection::{RootConnectionFactory, RootSpec},
    db::{CheckedHostRepository, SqliteCheckedHostRepository},
    model::{CheckedHost, PortLevel},
    service::port_checker::PortChecker,
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

pub struct HostCheckService<R, P> {
    repo: R,
    port_checker: P,
}

impl<R, P> HostCheckService<R, P> {
    pub fn new(repo: R, port_checker: P) -> Self {
        Self {
            repo,
            port_checker,
        }
    }
}

impl<R, P> HostCheckService<R, P>
where
    R: CheckedHostRepository,
    P: PortChecker,
{
    const TTL: Duration = Duration::from_hours(24);
    const HEALTH_CHECK_INTERVAL: chrono::TimeDelta = chrono::TimeDelta::seconds(10);

    /// ホストチェックの実装
    /// - `target_addr`: チェック対象のIPアドレス
    /// - `target_port`: チェック対象のポート番号
    /// - 戻り値: ホスト情報(CheckedHost)
    ///
    /// # Errors
    /// - `HostCheckServiceError::InternalDB`: データベース操作中にエラーが発生した場合
    /// - `HostCheckServiceError::Internal`: その他の内部エラーが発生した場合
    pub async fn do_check(&self, target_addr: IpAddr, target_port: u16) -> Result<CheckedHost, HostCheckServiceError> {
        let now = chrono::Utc::now();
        let host = self.repo.find_by_ip_port(target_addr, target_port).await?;

        let Some(host) = host else {
            // テーブルになかった場合、単にチェックして結果を保存して返す
            let result = self.port_checker.check(target_addr, target_port).await;
            let id = self.repo.insert(target_addr, target_port, result, None).await?;
            let host = self.repo.find_by_id(id).await?.ok_or_else(|| {
                anyhow!("Inserted a row and obtained its ID, but no row with that ID was found. id: {}", id)
            })?;
            return Ok(host);
        };

        // ポートが解放されていて、
        // 更新時＋TTLの合算時が現在時刻より小さいならば、そのままホスト情報を返してよい
        if host.port_level == PortLevel::Welldone && host.updated_at + Self::TTL >= now {
            // skip check
            return Ok(host);
        }

        // 最終チェックから時間が経過していない場合、そのままホスト情報を返す
        if now.signed_duration_since(host.updated_at) < Self::HEALTH_CHECK_INTERVAL {
            return Ok(host);
        }

        // ポートチェックして結果を保存して返す
        let mut host = host;
        let result_level = self.port_checker.check(target_addr, target_port).await;
        host.port_level = result_level;
        let _ = self.repo.update(&mut host).await?;

        Ok(host)
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeDelta;
    use mockall::predicate::{self, *};

    use crate::{
        db::MockCheckedHostRepository,
        model::DbIpAddr,
        service::port_checker::{self, MockPortChecker},
    };

    use super::*;

    // テーブルにホストがなく、pingを送って成功する場合
    #[tokio::test]
    async fn test_no_host_on_table() {
        let now = Utc::now();
        let ip_address: IpAddr = "127.0.0.1".parse().unwrap();
        let port = 7144;

        let mut repo = MockCheckedHostRepository::new();
        let mut port_checker = MockPortChecker::new();

        repo.expect_find_by_ip_port()
            //
            .with(predicate::eq(ip_address), predicate::eq(port))
            .times(1)
            .return_once(move |ip_address: IpAddr, port: u16| Ok(None));

        port_checker
            .expect_check()
            //
            .with(predicate::eq(ip_address), predicate::eq(port))
            .times(1)
            .return_once(|_, _| PortLevel::Welldone);

        // 結果をテーブルに登録
        repo.expect_insert()
            //
            .with(
                predicate::eq(ip_address),
                predicate::eq(port),
                predicate::eq(PortLevel::Welldone),
                predicate::eq(None),
            )
            .times(1)
            .return_once(|_, _, _, _| Ok(1000));

        repo.expect_find_by_id()
            //
            .with(predicate::eq(1000))
            .times(1)
            .return_once(move |_| {
                Ok(Some(CheckedHost {
                    id: Some(1000),
                    ip_address: DbIpAddr(ip_address),
                    port,
                    port_level: PortLevel::Welldone,
                    upload_speed: None,
                    updated_at: now,
                }))
            });

        let host = HostCheckService {
            repo,
            port_checker,
        }
        .do_check(ip_address, 7144)
        .await
        .unwrap();

        assert_eq!(host.ip_address.0, ip_address);
        assert_eq!(host.port, port);
        assert_eq!(host.port_level, PortLevel::Welldone);
        dbg!(host);
    }

    // テーブルにホストがなく、pingを送って失敗する場合
    #[tokio::test]
    async fn test_no_host_on_table_then_fail() {
        let now = Utc::now();
        let ip_address: IpAddr = "127.0.0.1".parse().unwrap();
        let port = 7144;

        let mut repo = MockCheckedHostRepository::new();
        let mut port_checker = MockPortChecker::new();

        repo.expect_find_by_ip_port()
            //
            .with(predicate::eq(ip_address), predicate::eq(port))
            .times(1)
            .return_once(move |ip_address: IpAddr, port: u16| Ok(None));

        port_checker
            .expect_check()
            //
            .with(predicate::eq(ip_address), predicate::eq(port))
            .times(1)
            .return_once(|_, _| PortLevel::Incomplete);

        // 結果をテーブルに登録
        repo.expect_insert()
            //
            .with(
                predicate::eq(ip_address),
                predicate::eq(port),
                predicate::eq(PortLevel::Incomplete),
                predicate::eq(None),
            )
            .times(1)
            .return_once(|_, _, _, _| Ok(1000));

        repo.expect_find_by_id()
            //
            .with(predicate::eq(1000))
            .times(1)
            .return_once(move |_| {
                Ok(Some(CheckedHost {
                    id: Some(1000),
                    ip_address: DbIpAddr(ip_address),
                    port,
                    port_level: PortLevel::Incomplete,
                    upload_speed: None,
                    updated_at: now,
                }))
            });

        let host = HostCheckService {
            repo,
            port_checker,
        }
        .do_check(ip_address, 7144)
        .await
        .unwrap();

        assert_eq!(host.ip_address.0, ip_address);
        assert_eq!(host.port, port);
        assert_eq!(host.port_level, PortLevel::Incomplete);
        dbg!(host);
    }

    // テーブルにWelldoneなホストが見つかって、有効期限内の場合
    // 値をそのまま返す
    #[tokio::test]
    async fn test_host_on_table_in_valid_cache() {
        let now = Utc::now();
        let ip_address: IpAddr = "127.0.0.1".parse().unwrap();

        let mut repo = MockCheckedHostRepository::new();
        repo.expect_find_by_ip_port()
            //
            .with(predicate::eq(ip_address), predicate::eq(7144))
            .times(1)
            .return_once(move |ip_address: IpAddr, port: u16| {
                Ok(Some(CheckedHost {
                    id: Some(1),
                    ip_address: DbIpAddr(ip_address),
                    port,
                    port_level: PortLevel::Welldone,
                    upload_speed: None,
                    updated_at: now.clone(),
                }))
            });

        let mut port_checker = MockPortChecker::new();
        // port_checker
        //     .expect_check()
        //     //
        //     .with(predicate::eq(ip_address), predicate::eq(7144))
        //     .times(1)
        //     .return_once(|_, _| PortLevel::Welldone);

        let host = HostCheckService {
            repo,
            port_checker,
        }
        .do_check(ip_address, 7144)
        .await
        .unwrap();
        // dbg!(host);
    }

    // テーブルにWelldoneなホストが見つかって、有効期限外の場合、
    // チェックポートを失敗して更新して値を返す
    #[tokio::test]
    async fn test_host_on_table_in_invalid_cache() {
        let now = Utc::now();
        let ip_address: IpAddr = "127.0.0.1".parse().unwrap();
        let port = 7144;

        let mut repo = MockCheckedHostRepository::new();
        let updated_at =
            now - (HostCheckService::<MockCheckedHostRepository, MockPortChecker>::TTL + Duration::from_hours(1));
        repo.expect_find_by_ip_port()
            //
            .with(predicate::eq(ip_address), predicate::eq(port))
            .times(1)
            .return_once(move |ip_address: IpAddr, port: u16| {
                Ok(Some(CheckedHost {
                    id: Some(1),
                    ip_address: DbIpAddr(ip_address),
                    port,
                    port_level: PortLevel::Welldone,
                    upload_speed: None,
                    // 期限切れの時刻を入れる
                    updated_at: updated_at,
                }))
            });

        let mut port_checker = MockPortChecker::new();
        port_checker
            .expect_check()
            //
            .with(predicate::eq(ip_address), predicate::eq(port))
            .times(1)
            .return_once(|_, _| PortLevel::Incomplete);

        // 更新が走るはず
        repo.expect_update()
            //
            .with(eq(CheckedHost {
                id: Some(1),
                ip_address: DbIpAddr(ip_address),
                port: port,
                port_level: PortLevel::Incomplete,
                upload_speed: None,
                updated_at: updated_at,
            }))
            .times(1)
            .return_once(|_| Ok(()));

        let host = HostCheckService {
            repo,
            port_checker,
        }
        .do_check(ip_address, port)
        .await
        .unwrap();
        // dbg!(host);
        assert_eq!(host.ip_address.0, ip_address);
        assert_eq!(host.port_level, PortLevel::Incomplete);
    }

    // テーブルにIncompleteなホストが見つかって、ポートチェック制限時間内の場合、
    // 何もせず値を返す
    #[tokio::test]
    async fn test_incomplete_host_on_table_in_invalid_check_time() {
        let now = Utc::now();
        let ip_address: IpAddr = "127.0.0.1".parse().unwrap();
        let port = 7144;

        let mut repo = MockCheckedHostRepository::new();
        let mut port_checker = MockPortChecker::new();

        // ヘルスチェックインターバル期間内の時間を設定する
        let updated_at = now
            - (HostCheckService::<MockCheckedHostRepository, MockPortChecker>::HEALTH_CHECK_INTERVAL
                - TimeDelta::seconds(5));
        repo.expect_find_by_ip_port()
            //
            .with(predicate::eq(ip_address), predicate::eq(port))
            .times(1)
            .return_once(move |ip_address: IpAddr, port: u16| {
                Ok(Some(CheckedHost {
                    id: Some(1),
                    ip_address: DbIpAddr(ip_address),
                    port,
                    port_level: PortLevel::Incomplete,
                    upload_speed: None,
                    updated_at: updated_at,
                }))
            });

        let host = HostCheckService {
            repo,
            port_checker,
        }
        .do_check(ip_address, port)
        .await
        .unwrap();
        // dbg!(host);
        assert_eq!(host.ip_address.0, ip_address);
        assert_eq!(host.port_level, PortLevel::Incomplete);
    }

    // テーブルにIncompleteなホストが見つかって、ポートチェック制限時間期限外の場合、
    // ポートチェックが走って、更新を行い値を返す
    #[tokio::test]
    async fn test_incomplete_host_on_table_in_valid_check_time() {
        let now = Utc::now();
        let ip_address: IpAddr = "127.0.0.1".parse().unwrap();
        let port = 7144;

        let mut repo = MockCheckedHostRepository::new();

        // ヘルスチェック制限時間外の時間を設定する
        let updated_at = now
            - (HostCheckService::<MockCheckedHostRepository, MockPortChecker>::HEALTH_CHECK_INTERVAL
                + TimeDelta::seconds(1));
        repo.expect_find_by_ip_port()
            //
            .with(predicate::eq(ip_address), predicate::eq(port))
            .times(1)
            .return_once(move |ip_address: IpAddr, port: u16| {
                Ok(Some(CheckedHost {
                    id: Some(1),
                    ip_address: DbIpAddr(ip_address),
                    port,
                    port_level: PortLevel::Incomplete,
                    upload_speed: None,
                    updated_at: updated_at,
                }))
            });

        let mut port_checker = MockPortChecker::new();
        // ポートチェック
        port_checker
            .expect_check()
            //
            .with(predicate::eq(ip_address), predicate::eq(port))
            .times(1)
            .return_once(|_, _| PortLevel::Welldone);

        // 更新
        repo.expect_update()
            //
            .with(eq(CheckedHost {
                id: Some(1),
                ip_address: DbIpAddr(ip_address),
                port: port,
                port_level: PortLevel::Welldone,
                upload_speed: None,
                updated_at: updated_at,
            }))
            .times(1)
            .return_once(|_| Ok(()));

        let host = HostCheckService {
            repo,
            port_checker,
        }
        .do_check(ip_address, port)
        .await
        .unwrap();
        // dbg!(host);
        assert_eq!(host.ip_address.0, ip_address);
        assert_eq!(host.port_level, PortLevel::Welldone);
    }

    // テーブルにIncompleteなホストが見つかって、ポートチェック制限時間期限外の場合、
    // ポートチェックが走って（Incomplete）、更新を行い値を返す
    #[tokio::test]
    async fn test_incomplete_host_on_table_in_valid_check_time_incomplete_ping() {
        let now = Utc::now();
        let ip_address: IpAddr = "127.0.0.1".parse().unwrap();
        let port = 7144;

        let mut repo = MockCheckedHostRepository::new();

        // ヘルスチェック制限時間外の時間を設定する
        let updated_at = now
            - (HostCheckService::<MockCheckedHostRepository, MockPortChecker>::HEALTH_CHECK_INTERVAL
                + TimeDelta::seconds(1));
        repo.expect_find_by_ip_port()
            //
            .with(predicate::eq(ip_address), predicate::eq(port))
            .times(1)
            .return_once(move |ip_address: IpAddr, port: u16| {
                Ok(Some(CheckedHost {
                    id: Some(1),
                    ip_address: DbIpAddr(ip_address),
                    port,
                    port_level: PortLevel::Incomplete,
                    upload_speed: None,
                    updated_at: updated_at,
                }))
            });

        let mut port_checker = MockPortChecker::new();
        // ポートチェック
        port_checker
            .expect_check()
            //
            .with(predicate::eq(ip_address), predicate::eq(port))
            .times(1)
            .return_once(|_, _| PortLevel::Incomplete);

        // 更新
        repo.expect_update()
            //
            .with(eq(CheckedHost {
                id: Some(1),
                ip_address: DbIpAddr(ip_address),
                port: port,
                port_level: PortLevel::Incomplete,
                upload_speed: None,
                updated_at: updated_at,
            }))
            .times(1)
            .return_once(|_| Ok(()));

        let host = HostCheckService {
            repo,
            port_checker,
        }
        .do_check(ip_address, port)
        .await
        .unwrap();
        // dbg!(host);
        assert_eq!(host.ip_address.0, ip_address);
        assert_eq!(host.port_level, PortLevel::Incomplete);
    }
}
