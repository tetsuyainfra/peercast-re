use tracing::info;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> () {
    let file_appender = tracing_appender::rolling::hourly("./tmp", "prefix.log");
    let pretty_layer = fmt::layer().pretty().with_writer(std::io::stdout);
    let json_layer = fmt::layer().json().with_writer(file_appender);

    // tracing_subscriber::registry() is an alias for Registry::default()
    tracing_subscriber::registry()
        //
        .with(json_layer)
        .with(pretty_layer)
        .init(); // Initialize the tracing subscriber( tracing::set_global_default(registry) と同じ )

    info!("Tracing initialized");
    info!(target = "access_log", "This is a test log message");
}
