use std::net::IpAddr;

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::model::CheckedHost;
use crate::model::DbIpAddr;
use crate::model::PortLevel;
use crate::prelude::*;

#[cfg_attr(test, mockall::automock)]
#[async_trait::async_trait]
pub trait CheckedHostRepository {
    async fn migrate(&self) -> Result<(), sqlx::Error>;
    // async fn add_checked_host(&self, host: &str) -> Result<(), String>;
    // async fn remove_checked_host(&self, host: &str) -> Result<(), String>;
    async fn all(&self) -> Result<Vec<CheckedHost>, sqlx::Error>;
    async fn find_by_id(&self, id: i64) -> Result<Option<CheckedHost>, sqlx::Error>;
    async fn find_by_ip_port(&self, ip: IpAddr, port: u16) -> Result<Option<CheckedHost>, sqlx::Error>;
    async fn find_all_by_ip(&self, ip: IpAddr) -> Result<Vec<CheckedHost>, sqlx::Error>;
    /// 新規にレコードを追加し、そのIDを返す
    async fn insert(
        &self,
        ip: IpAddr,
        port: u16,
        port_level: PortLevel,
        port_speed: Option<u16>,
    ) -> Result<i64, sqlx::Error>;
    async fn update(&self, host: &CheckedHost) -> Result<(), sqlx::Error>;
    async fn delete(&self, id: i64) -> Result<(), sqlx::Error>;
}

//////////////////////////////////////////////////////////////////////////////
// SQLite Implementations
pub struct SqliteCheckedHostRepository {
    pool: SqlitePool,
}

impl SqliteCheckedHostRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
        }
    }
}

#[async_trait::async_trait]
impl CheckedHostRepository for SqliteCheckedHostRepository {
    async fn migrate(&self) -> Result<(), sqlx::Error> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        info!("CheckedHost Database migrated successfully.");
        Ok(())
    }
    async fn all(&self) -> Result<Vec<CheckedHost>, sqlx::Error> {
        let hosts = sqlx::query_as!(
            CheckedHost,
            r#"SELECT id,
                ip_address AS `ip_address: DbIpAddr`,
                port as `port!:u16` ,
                port_level as `port_level: PortLevel`,
                upload_speed as `upload_speed: u16`,
                updated_at as `updated_at: DateTime<Utc>`
                    FROM checked_hosts
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(hosts)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<CheckedHost>, sqlx::Error> {
        let host = sqlx::query_as!(
            CheckedHost,
            r#"SELECT id,
                ip_address AS `ip_address: DbIpAddr`,
                port as `port!:u16` ,
                port_level as `port_level: PortLevel`,
                upload_speed as `upload_speed: u16`,
                updated_at as `updated_at: DateTime<Utc>`
               FROM checked_hosts WHERE id = $1"#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(host)
    }

    async fn find_by_ip_port(&self, ip: IpAddr, port: u16) -> Result<Option<CheckedHost>, sqlx::Error> {
        let ip = DbIpAddr(ip);
        let host = sqlx::query_as!(
            CheckedHost,
            r#"SELECT id,
                ip_address AS `ip_address: DbIpAddr`,
                port as `port!:u16`,
                port_level as `port_level: PortLevel`,
                upload_speed as `upload_speed: u16`,
                updated_at as `updated_at: DateTime<Utc>`
                    FROM checked_hosts
                    WHERE ip_address = $1 AND port = $2
            "#,
            ip,
            port
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(host)
    }

    /// ポート指定なしでIPアドレスのみで検索
    async fn find_all_by_ip(&self, ip: IpAddr) -> Result<Vec<CheckedHost>, sqlx::Error> {
        let ip = DbIpAddr(ip);
        let host = sqlx::query_as!(
            CheckedHost,
            r#"SELECT id,
                ip_address AS `ip_address: DbIpAddr`,
                port as `port!:u16` ,
                port_level as `port_level: PortLevel`,
                upload_speed as `upload_speed: u16`,
                updated_at as `updated_at: DateTime<Utc>`
                    FROM checked_hosts WHERE ip_address = $1
            "#,
            ip
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(host)
    }

    async fn insert(
        &self,
        ip: IpAddr,
        port: u16,
        port_level: PortLevel,
        upload_speed: Option<u16>,
    ) -> Result<i64, sqlx::Error> {
        let ip = DbIpAddr(ip);
        let id = sqlx::query_scalar!(
            r#"
            INSERT
                INTO checked_hosts (ip_address, port, port_level, upload_speed)
                VALUES ($1, $2, $3, $4)
                RETURNING id
            "#,
            ip,
            port,
            port_level,
            upload_speed
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(id)
    }

    async fn update(&self, host: &CheckedHost) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"UPDATE checked_hosts
                SET
                    ip_address = $1,
                    port = $2,
                    port_level = $3,
                    upload_speed = $4,
                    updated_at = CURRENT_TIMESTAMP
                WHERE id = $5
            "#,
            // set
            host.ip_address,
            host.port,
            host.port_level,
            host.upload_speed,
            // where
            host.id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query!(r#"DELETE FROM checked_hosts WHERE id = $1"#, id).execute(&self.pool).await?;

        Ok(())
    }
}

//////////////////////////////////////////////////////////////////////////////
// Postgres Implementations
// pub struct PgCheckedHostRepository{
//     pool: PgPool,
// }

#[cfg(test)]
mod tests {
    use std::net::IpAddr;

    use super::*;
    use sqlx::SqlitePool;

    async fn prepare_sqlite_repo() -> SqliteCheckedHostRepository {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        let _ = sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        SqliteCheckedHostRepository::new(pool)
    }

    async fn prepare_sqlite_repo_zero() -> SqliteCheckedHostRepository {
        let repo = prepare_sqlite_repo().await;
        let all = repo.all().await.unwrap();
        for host in all {
            repo.delete(host.id.unwrap()).await.unwrap()
        }
        assert_eq!(repo.all().await.unwrap().len(), 0);
        repo
    }

    mod add_checked_host {
        use super::*;
        #[tokio::test]
        async fn test_all() {
            let repo = prepare_sqlite_repo().await;
            let all = repo.all().await.unwrap();
            assert_eq!(all.len(), 7);
        }

        #[tokio::test]
        async fn test_insert() {
            let repo = prepare_sqlite_repo_zero().await;

            let ip = "127.0.0.1".parse::<IpAddr>().unwrap();
            let port = 8080;
            let port_level = PortLevel::Welldone;
            let port_speed = Some(2);

            let id = repo.insert(ip, port, port_level, port_speed).await.unwrap();
            let hosts = repo.find_all_by_ip(ip).await.unwrap();
            assert_eq!(hosts.len(), 1);
            assert_eq!(hosts[0].ip_address, DbIpAddr(ip));
            assert_eq!(hosts[0].port, port);
            assert_eq!(hosts[0].port_level, port_level);
            assert_eq!(hosts[0].upload_speed, port_speed);

            assert!(repo.insert(ip, port, port_level, port_speed).await.is_err());

            let r = repo.find_by_id(id).await.unwrap().unwrap();
            assert_eq!(r.ip_address, DbIpAddr(ip));
            assert_eq!(r.port, port);
            assert_eq!(r.port_level, port_level);
            assert_eq!(r.upload_speed, port_speed);
        }
        #[tokio::test]
        async fn test_update() {
            let repo = prepare_sqlite_repo_zero().await;

            let ip = "127.0.0.1".parse::<IpAddr>().unwrap();
            let port = 8080;
            let port_level = PortLevel::Incomplete;
            let port_speed = None;

            let id = repo.insert(ip, port, port_level, port_speed).await.unwrap();
            let mut host = repo.find_by_id(id).await.unwrap().unwrap();
            host.ip_address = DbIpAddr("127.0.0.2".parse::<IpAddr>().unwrap());
            host.port = 8081;
            host.port_level = PortLevel::Welldone;
            host.upload_speed = Some(100);
            repo.update(&host).await.unwrap();

            let diff_host = repo.find_by_id(id).await.unwrap().unwrap();
            assert_eq!(diff_host.id, host.id);
            assert_eq!(diff_host.ip_address, host.ip_address);
            assert_eq!(diff_host.port, host.port);
            assert_eq!(diff_host.port_level, host.port_level);
        }
    }
}
