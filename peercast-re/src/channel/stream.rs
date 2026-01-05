use std::task::Waker;

use futures::Stream;
use libpeercast_re::pcp::GnuId;

use crate::prelude::*;

pub struct ReStream {
    cid: GnuId,
    count: usize,
    last_update: std::time::Instant,
    tx: tokio::sync::mpsc::UnboundedSender<(Waker, std::time::Duration)>,
}

impl ReStream {
    const DATA_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);
    pub async fn new(cid: GnuId) -> Self {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(Waker, std::time::Duration)>();
        // let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let _ = tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Some((waker, delta)) => {
                        // データが到着した場合、wakerを起動する
                        info!("ReStream waiter woke up after {:?} for channel {}", delta, cid);
                        tokio::time::sleep(Self::DATA_INTERVAL - delta).await;
                        waker.wake();
                    }
                    None => break, // チャンネルが閉じられた場合、ループを終了
                }
            }
        });

        Self {
            cid,
            count: 0,
            last_update: std::time::Instant::now(),
            tx,
        }
    }
}

impl Stream for ReStream {
    type Item = Result<bytes::Bytes, std::io::Error>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let delta = self.last_update.elapsed();
        if delta < Self::DATA_INTERVAL {
            // info!("No new data yet for channel {}", self.cid);
            self.tx.send((cx.waker().clone(), delta)).ok();
            return std::task::Poll::Pending;
        }
        info!("Polling new data for channel {}", self.cid);

        // ここでストリームからデータを取得し、ポーリングします
        while self.count < 5 {
            let this = self.get_mut();

            this.count += 1;
            this.last_update = std::time::Instant::now();

            let data = format!("Data chunk {} from channel {}\n", this.count, this.cid);
            let bytes = bytes::Bytes::from(data);
            // 通常はここで非同期にデータを取得しますが、今回は簡単のために即座に返します

            return std::task::Poll::Ready(Some(Ok(bytes)));
        }
        std::task::Poll::Ready(None) // 仮の実装
    }
}
