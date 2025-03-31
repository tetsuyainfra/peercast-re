use std::time::Duration;

use tokio::signal;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("process number: {}", std::process::id());
    let graceful_token = CancellationToken::new();
    let child_token = graceful_token.child_token();

    let shutdown_handle = tokio::spawn(async move {
        #[cfg(unix)]
        let terminate = async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = signal::ctrl_c() => {
                println!("catch ctrl_c");
                graceful_token.cancel();
            }
            _ = terminate =>  {
                println!("catch terminate");
            }
        }
    });

    let tracker = tokio_util::task::TaskTracker::new();
    let _main_handle = tracker.spawn(async move {
        let child_tracker = tokio_util::task::TaskTracker::new();

        let child_child_token = child_token.child_token();
        child_tracker.spawn(async move {
            tokio::select! {
                _ = child_child_token.cancelled() => {
                    println!("childchild canceled")
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
            println!("childchild finished")
        });

        tokio::select! {
            _ = child_token.cancelled() => {
                println!("child canceled")
            }
            _ = tokio::time::sleep(Duration::from_secs(5)) => {
                println!("child sleep out")
            }
        }

        child_tracker.close();
        child_tracker.wait().await;
        println!("child finished");
    });

    tracker.close();
    tracker.wait().await;

    Ok(())
}
