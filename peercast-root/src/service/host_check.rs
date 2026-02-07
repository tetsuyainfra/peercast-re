use crate::{PortLevel, db::CheckedHost};


pub struct HostCheckService;

impl  HostCheckService{
    // ホストチェックの実装
    async fn do_host_check() -> Result<PortLevel, Box<dyn std::error::Error>> {
        // DBにホスト情報があるか確認

        let host : CheckedHost = unimplemented!();
        // 期限が切れていなくてポートが有効な場合は早期リターン
        // if host.created_at + host.valid_duration > chrono::Utc::now().naive_utc() {
        // }


        // 期限が切れて場合は再チェック

        // ポートチェックを実施


    }
}
