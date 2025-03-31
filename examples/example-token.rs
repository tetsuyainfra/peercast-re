fn main() {}

#[cfg(test)]
mod t {
    use std::time::Duration;

    use tokio::time::sleep;
    use tokio_util::sync::CancellationToken;

    // cancelしたらすべてキャンセルされるまで末みたいな処理がしたい
    #[tokio::test]
    async fn test_dropwait() {
        let tracker = tokio_util::task::TaskTracker::new();
        let token = CancellationToken::new();
        let child_token = token.child_token();
        let h1 = tracker.spawn(async move {
            sleep(Duration::from_secs(1)).await;
            child_token.cancelled().await;
            println!("children cancelled");
            std::future::pending::<()>().await;
        });

        // cancelして・・・
        token.cancel();

        // すべてのtaskの終了を待つ
        tracker.close();
        tokio::select! {
            _ = tracker.wait() => {}
            _ = tokio::signal::ctrl_c() => {
                println!("catch ctrl_c");
                h1.abort();
            }
        }

        println!("finished");
    }

    // 兄弟tokenに作用はしない
    #[tokio::test]
    async fn test_kyodai() {
        let token = CancellationToken::new();
        let child_token = token.child_token();
        let h1 = tokio::spawn(async move {
            sleep(Duration::from_secs(1)).await;
            child_token.cancel();
        });

        let child_token = token.child_token();
        let h2 = tokio::spawn(async move {
            tokio::select! {
                _ = std::future::pending::<()>() => {}
                _ = child_token.cancelled() => {}
            };
        });

        futures_util::future::join_all(vec![h1, h2]).await;
    }
}
