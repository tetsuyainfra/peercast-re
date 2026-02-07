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
    async fn find_by_ip(&self, ip: &DbIpAddr) -> Result<Option<CheckedHost>, sqlx::Error>;
    async fn find_by_id(&self, id: i64) -> Result<Option<CheckedHost>, sqlx::Error>;
    async fn add(&self, host: &CheckedHost) -> Result<(), sqlx::Error>;
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
            "SELECT id, ip AS `ip: DbIpAddr`, created_at as `created_at: DateTime<Utc>`FROM checked_hosts"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(hosts)
    }

    async fn find_by_ip(&self, ip: &DbIpAddr) -> Result<Option<CheckedHost>, sqlx::Error> {
        let host = sqlx::query_as!(
            CheckedHost,
            "SELECT id, ip AS `ip: DbIpAddr`, created_at `created_at: DateTime<Utc>` FROM checked_hosts WHERE ip = $1",
            ip
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(host)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<CheckedHost>, sqlx::Error> {
        let host = sqlx::query_as!(
            CheckedHost,
            "SELECT id, ip AS `ip: DbIpAddr`, created_at as `created_at: DateTime<Utc>` FROM checked_hosts WHERE id = $1",
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(host)
    }

    async fn add(&self, host: &CheckedHost) -> Result<(), sqlx::Error> {
        sqlx::query_scalar!(
            r#"
            INSERT INTO checked_hosts (ip) VALUES ($1)
                ON CONFLICT(ip)
                    DO UPDATE SET created_at = CURRENT_TIMESTAMP;
            "#,
            host.ip
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
        assert_eq!(all.len(), 3);

        let ip: IpAddr = "255.0.0.2".parse().unwrap();
        let r = repo.find_by_ip(&DbIpAddr(ip)).await;
        assert!(r.ok().is_some());
    }
}
