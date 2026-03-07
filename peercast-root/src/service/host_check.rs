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
            port_level: PortLevel::Incomplete,
            upload_speed: None,
            updated_at: Utc::now(),
        }
    }
}
