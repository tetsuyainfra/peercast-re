use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

use crate::cli;

pub fn init(args: &cli::Args) -> anyhow::Result<()> {
    // println!("CARGO_CRATE_NAME: {}", env!("CARGO_CRATE_NAME"));
    // println!("CARGO_BIN_NAME: {}", env!("CARGO_BIN_NAME"));
    println!("args-> {:#?}", args);

    // for tokio-console
    // console_subscriber::init();
    let console_layer = console_subscriber::ConsoleLayer::builder()
        .with_default_env()
        .spawn();

    // STDOUT
    let fmt_filter = if args.verbose.is_present() == false {
        // 通常時 (RUST_LOGの指定があればそれを読み込む)
        tracing_subscriber::filter::EnvFilter::try_from_default_env().unwrap_or_else(|e| {
            // let v = std::env::var("RUST_LOG").unwrap_or_default();
            // eprintln!(
            //     "Faild to initialize logging filter from RUST_LOG=\"{}\", {}",
            //     v, e
            // );
            "info".into()
        })
    } else {
        // コマンドラインオプションで指定があった場合
        EnvFilter::builder()
            .with_default_directive(args.verbose.tracing_level_filter().into())
            .parse("")
            .unwrap()
    };
    let fmt_filter_str = fmt_filter.to_string();

    // ACCESS LOG
    let file =
        std::fs::File::create(args.access_log.clone()).expect("access_logの作成に失敗しました");
    let access_log = tracing_subscriber::fmt::layer()
        // .with_thread_names(true)
        .json()
        // .with_target(true)
        .with_writer(file);
    let access_log_filter_fn =
        tracing_subscriber::filter::filter_fn(|metadata| metadata.target() == "http_access");

    // CONSOLE OUTPUT
    let fmt_layer = tracing_subscriber::fmt::layer();
    tracing_subscriber::registry()
        .with(console_layer)
        .with(fmt_layer.with_filter(fmt_filter))
        .with(access_log.with_filter(access_log_filter_fn))
        // .with(access_log)
        .init();

    tracing::info!("EnvFilter setting: {}", fmt_filter_str);

    Ok(())
}

/*
fn init_subscriber<L, F, B>(v: &clap_verbosity_flag::Verbosity<L>, f: F)
where
    L: clap_verbosity_flag::LogLevel,
    F: FnOnce(
        SubscriberBuilder<
            tracing_subscriber::fmt::format::DefaultFields,
            tracing_subscriber::fmt::format::Format<Full, LocalTime<Rfc3339>>,
            EnvFilter,
        >,
    ) -> B,
    B: SubscriberInitExt,
{
    use tracing_subscriber::filter::LevelFilter;
    tracing_subscriber::fmt::time;

    // match v.log_level_filter().as_trace() {
    match v.tracing_level_filter() {
        LevelFilter::OFF => (),
        filter => {
            let env_filter = into_env_filter(filter);
            let builder = SubscriberBuilder::default()
                .with_timer(time::LocalTime::rfc_3339())
                .with_env_filter(env_filter);
            f(builder).init();
            // f(SubscriberBuilder::default()).init();
        }
    }
}

fn into_env_filter(filter: tracing_subscriber::filter::LevelFilter) -> EnvFilter {
    use tracing::Level;
    let crate_name = env!("CARGO_CRATE_NAME");
    // If log level is lower than debug, only apply it to CRATES targets.
    let default = if filter >= Level::DEBUG {
        format!("info,{crate_name}={filter}",)
    } else {
        filter.to_string()
    };

    // println!("crate_name: {}", crate_name);
    // println!("into env filter: {}", default);
    EnvFilter::try_from_default_env().unwrap_or_else(|_| default.into())
}

fn test() {
    let v = Verbosity::<ErrorLevel>::new(0, 0);
    init_subscriber(&v, |b| b.with_ansi(false));
}
*/
