use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use tokio::signal;
use tokio::time::{Duration, sleep};
use tokio_util::sync::CancellationToken;



#[cfg(test)]
mod tests {
    use tokio_util::task::TaskTracker;

    use super::*;
    async fn wait_for_cancellation(no: usize, token: CancellationToken) {
        println!("no: {no} started");
        tokio::select! {
            _ = token.cancelled() => {
                println!("no: {no} cancelled");
            }
        }
        println!("no: {no} finished");
    }

    #[tokio::test]
    async fn test_wait_for_cancellation() {
        let mut tracker = TaskTracker::new();
        let mut token = CancellationToken::new();

        let mut handles = Vec::new();
        for i in 0..2  {
            println!("{i}");
            let handle = tracker.spawn(wait_for_cancellation(i, token.child_token()));
            handles.push(handle);
        }

        // Realtimeにタスクを追加できない
        tracker.close();

        let _handle = tokio::spawn(async move {
            println!("Wait for 2 seconds before cancelling");
            sleep(Duration::from_secs(2)).await;
            token.cancel();
            println!("Cancellation token cancelled");
        });

        tracker.wait().await;
        println!("All tasks completed");
    }
}
