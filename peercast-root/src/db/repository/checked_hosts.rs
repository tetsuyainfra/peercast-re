use std::net::IpAddr;

use async_trait::async_trait;
use sqlx::SqlitePool;
use chrono::{DateTime, Utc};

use crate::prelude::*;
use crate::db::model::CheckedHost;
use crate::db::model::DbIpAddr;

#[async_trait]
pub trait CheckedHostRepository {
    async fn migrate(&self) -> Result<(), sqlx::Error> ;
    // async fn add_checked_host(&self, host: &str) -> Result<(), String>;
    // async fn remove_checked_host(&self, host: &str) -> Result<(), String>;
    async fn all(&self) -> Result<Vec<CheckedHost>, sqlx::Error> ;
    async fn find_by_id(&self, id: i64) -> Result<Option<CheckedHost>, sqlx::Error>;
    async fn find_by_ip_port(&self, ip: IpAddr, port: u16) -> Result<Option<CheckedHost>, sqlx::Error>;
    async fn find_all_by_ip(&self, ip: IpAddr) -> Result<Vec<CheckedHost>, sqlx::Error>;
    async fn add(&self, ip: IpAddr, port: u16, speed: i32) -> Result<(), sqlx::Error>;
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

#[async_trait]
impl CheckedHostRepository for SqliteCheckedHostRepository {
    async fn migrate(&self) -> Result<(), sqlx::Error> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        info!("CheckedHost Database migrated successfully.");
        Ok(())
    }
    async fn all(&self) -> Result<Vec<CheckedHost>, sqlx::Error> {
        let hosts = sqlx::query_as!(
            CheckedHost,
            "SELECT id, ip_address AS `ip_address: DbIpAddr`, port as `port!:u16` , speed as `speed!:i32`, created_at as `created_at: DateTime<Utc>`FROM checked_hosts"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(hosts)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<CheckedHost>, sqlx::Error> {
        let host = sqlx::query_as!(
            CheckedHost,
            "SELECT id, ip_address AS `ip_address: DbIpAddr`, port as `port!:u16` , speed as `speed!:i32`, created_at as `created_at: DateTime<Utc>` FROM checked_hosts WHERE id = $1",
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
            "SELECT id, ip_address AS `ip_address: DbIpAddr`, port as `port!:u16` , speed as `speed!:i32`, created_at `created_at: DateTime<Utc>` FROM checked_hosts WHERE ip_address = $1 AND port = $2",
            ip,
            port
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(host)
    }

    async fn find_all_by_ip(&self, ip: IpAddr) -> Result<Vec<CheckedHost>, sqlx::Error> {
        let ip = DbIpAddr(ip);
        let host = sqlx::query_as!(
            CheckedHost,
            "SELECT id, ip_address AS `ip_address: DbIpAddr`, port as `port!:u16` , speed as `speed!:i32`, created_at `created_at: DateTime<Utc>` FROM checked_hosts WHERE ip_address = $1",
            ip
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(host)
    }

    async fn add(&self, ip: IpAddr, port: u16, speed: i32) -> Result<(), sqlx::Error> {
        let ip = DbIpAddr(ip);
        sqlx::query_scalar!(
            r#"
            INSERT INTO checked_hosts (ip_address, port, speed) VALUES ($1, $2, $3)
                ON CONFLICT(ip_address, port)
                    DO UPDATE SET created_at = CURRENT_TIMESTAMP;
            "#,
            ip,
            port,
            speed
        )
        .execute(&self.pool)
        .await?;

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

    #[tokio::test]
    async fn test_add_checked_host()  {
        let repo = prepare_sqlite_repo().await;

        let all = repo.all().await.unwrap();
        assert_eq!(all.len(), 7);

        let ip: IpAddr = "255.0.0.2".parse().unwrap();
        let r = repo.find_all_by_ip(ip).await;
        assert!(r.ok().is_some());
    }
}
