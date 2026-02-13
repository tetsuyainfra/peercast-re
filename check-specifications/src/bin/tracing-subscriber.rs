#[allow(unused_imports)]
use tracing::{Level, debug, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = tracing_subscriber::fmt()
        // filter spans/events with level TRACE or higher.
        .with_max_level(Level::TRACE)
        // build but do not install the subscriber.
        .finish();

    // Subscriberが有効なスコープ内でのみログが出力される
    tracing::subscriber::with_default(subscriber, || {
        info!("This will be logged to stdout");
    });

    // ここではSubscriberが有効でないためログは出力されない
    info!("This will _not_ be logged to stdout");

    tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        // .init()を呼び出してグローバルにSubscriberを設定できる
        .init();

    info!("This will be logged to stdout as global");

    Ok(())
}

/*
https://docs.rs/tracing/latest/tracing/
https://docs.rs/tracing/latest/tracing/#in-libraries

https://docs.rs/tracing-subscriber/latest/tracing_subscriber/
https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/

https://docs.rs/tracing-appender/latest/tracing_appender/
https://docs.rs/tracing-journald/latest/tracing_journald/
*/
