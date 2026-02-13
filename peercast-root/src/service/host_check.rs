use std::net::{IpAddr, SocketAddr};

use chrono::Duration;

use crate::{db::{CheckedHostRepository,  SqliteCheckedHostRepository}, model::PortLevel};

#[derive(thiserror::Error, Debug)]
pub enum HostCheckServiceError {
    #[error("Database error")]
    InternalDB(#[from] sqlx::Error),

    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

pub struct HostCheckService;

impl HostCheckService {
    const CHECK_VALID_HOURS: i64 = 24;
    const CHECK_INTERVAL_SECS: i64 = 30;

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
    ///    2.b ポート開放済みかつ有効期限切れの場合は再度ポートチェックを実施
    ///    2.c ポート未開放の場合はポートチェックを実施、ただし有効期限内の場合は早期リターン
    ///    2.d ホスト情報が存在しない場合は新規にポートチェックを実施
    /// 3. ポートチェックは最終実行から一定時間経過している場合にのみ実施
    /// 4. ポートチェックの結果に基づいてデータベースを更新し、最終的なPortLevelを返す。
    pub async fn do_host_check(
        db_pool: &sqlx::Pool<sqlx::sqlite::Sqlite>,
        target_addr: IpAddr,
        target_port: u16,
    // ) -> Result<PortLevel, HostCheckServiceError> {
    ) -> Result<PortLevel, HostCheckServiceError> {
        let checked_host_repo = SqliteCheckedHostRepository::new(db_pool.clone());

        let host = checked_host_repo.find_by_ip_port(target_addr, target_port).await?;

        let now = chrono::Utc::now();

        if host.is_none() {
            // ホスト情報が存在しない場合、新規にポートチェックを実施
            // ポートチェックの実装をここに追加
            return todo!();
        }

        let host = host.unwrap();
        let valid_duration = Duration::hours(Self::CHECK_VALID_HOURS);
        // if host.port_level as i16 <= 0  {

        // } else {

        // }

        // ポートチェックを実施
        todo!()
    }
}
