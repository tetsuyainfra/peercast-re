/*! example-daemon: daemonの動作を簡易的に示したもの
 * -v warnを表示, -vv infoを表示, .. , -vvvv traceを表示 (標準ではerror以上を表示)
 * -D オプションでdaemonモードで動作する(stdout, stderrをリダイレクト等したもの）。systemd環境では指定しなくても良い(stdoutをそのままログに残してくれるので)
* stdout 処理上の情報 + オプションでaccess.log
* stderr 処理上の情報 + オプションでerror.log
* access.log
* error.log
* DEBUG:
*  `tokio-console` を実行すればタスクの実行状態がわかる
* RemoteIPを取り込む
* SEE:
*  - Logging
*    - https://github.com/tokio-rs/axum/blob/main/examples/tracing-aka-logging/src/main.rs
*    - https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/index.html
*  - Request-ID
*    - HTTPリクエストに含まれていたらその値を使う（そのため完全に信頼できる一意な値ではない）
*      - システムで一意に吐き出されるIDだとgithub-delivery-id等がある
*    - https://github.com/tokio-rs/axum/tree/main/examples/request-id
*  - Shutdown
*    - https://github.com/tokio-rs/axum/tree/main/examples/tls-graceful-shutdown
*/

use std::{env, net::SocketAddr, process::exit, time::Duration};

use axum::{
    extract::{MatchedPath, Request},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use axum_client_ip::{SecureClientIp, SecureClientIpSource};
use clap::Parser;
use daemonize::Daemonize;
use tower::ServiceBuilder;
use tower_http::{classify::ServerErrorsFailureClass, request_id, trace::TraceLayer};
use tracing::{error, info, info_span, Span};

mod cli;
mod logging;
mod remote_ip;

const REQUEST_ID_HEADER: &str = "x-request-id";

fn main() -> anyhow::Result<()> {
    let args = cli::Args::parse();
    match args.command {
        Some(cli::Commands::Version { json }) => {
            version_print(json)?;
            exit(0);
        }
        None => (),
    };

    if args.daemon {
        daemonize(args)
    } else {
        run(args, "Normal Run")
    }
}

fn daemonize(args: cli::Args) -> anyhow::Result<()> {
    let stdout = std::fs::File::create("./temp/daemon.out").unwrap();
    let stderr = std::fs::File::create("./temp/daemon.err").unwrap();

    let daemonize = Daemonize::new()
        .working_directory("./temp/daemon-dir")
        .pid_file("test.pid")
        .stdout(stdout)
        .stderr(stderr)
        .privileged_action(|| return "Executed before drop privileges");

    match daemonize.start() {
        Ok(v) => {
            println!("Success, daemonized");
            println!("v: {:?}", v);
            run(args, "Demonized Run")
        }
        Err(e) => {
            eprintln!("Error, {}", e);
            anyhow::bail!("");
        }
    }
}

fn run(args: cli::Args, runned_by: &str) -> anyhow::Result<()> {
    println!("{}", runned_by);
    println!("pwd: {:?}", std::env::current_dir().unwrap());

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .thread_name("example-daemon-thread")
        .thread_stack_size(3 * 1024 * 1024)
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(_run(args))
}

async fn _run(args: cli::Args) -> anyhow::Result<()> {
    logging::init(&args)?;

    let ip_source = SecureClientIpSource::ConnectInfo;
    let x_request_id_name = http::HeaderName::from_static(REQUEST_ID_HEADER);

    let service_layer = ServiceBuilder::new()
        .layer(remote_ip::SetRemoteIpLayer::new(ip_source.clone()))
        .layer(tower_http::request_id::SetRequestIdLayer::new(
            x_request_id_name.clone(),
            tower_http::request_id::MakeRequestUuid,
        ))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<_>| {
                    // let request_id = request.headers().get(REQUEST_ID_HEADER);
                    let request_id = request.extensions().get::<request_id::RequestId>();
                    let remote_addr = request.extensions().get::<axum_client_ip::SecureClientIp>();

                    // Log the matched route's path (with placeholders not filled in).
                    // Use request.uri() or OriginalUri if you want the real path.
                    let matched_path = request
                        .extensions()
                        .get::<MatchedPath>()
                        .map(MatchedPath::as_str);

                    match request_id {
                        Some(request_id) => {
                            info_span!(
                                "http_request",
                                ?request_id,
                                method = ?request.method(),
                                remote_addr = ?remote_addr,
                                matched_path,
                                some_other_field = tracing::field::Empty,
                            )
                        }
                        None => {
                            error!("could not extract request_id");
                            info_span!(
                                "http_request",
                                method = ?request.method(),
                                remote_addr = ?remote_addr,
                                matched_path,
                                some_other_field = tracing::field::Empty,
                            )
                        }
                    }
                })
                // .on_request(|request: &Request<_>, span: &Span| {
                //     // You can use `_span.record("some_other_field", value)` in one of these
                //     // closures to attach a value to the initially empty field in the info_span
                //     // created above.
                //     tracing::debug!(target: "http_access", "started {} {}", request.method(), request.uri().path())
                // })
                .on_response(|response: &Response, latency: Duration, _span: &Span| {
                    let remote_addr = response.extensions().get::<SecureClientIp>();

                    let request_id = response.headers().get(REQUEST_ID_HEADER);

                    // UUIDだと外部からX-REQUEST-IDを与えられた時にパースできない可能性がある
                    // let request_id = request_id
                    //     .and_then(|v| v.to_str().ok())
                    //     .and_then(|s| s.parse::<uuid::Uuid>().ok());
                    let request_id = request_id.and_then(|v| v.to_str().ok());

                    match request_id {
                        Some(request_id) => {
                            tracing::info!(
                                target: "http_access",
                                request_id,
                                ?remote_addr,
                                status_code = ?response.status().as_u16(),
                                ?latency,
                            )
                        }
                        None => {
                            error!("could not extract request_id");
                            tracing::info!(
                                target: "http_access",
                                remote_addr = ?remote_addr,
                                status_code = ?response.status().as_u16(),
                                latency = ?latency
                            )
                        }
                    }
                })
                //         .on_body_chunk(|_chunk: &Bytes, _latency: Duration, _span: &Span| {
                //             // ...
                //         })
                //         .on_eos(
                //             |_trailers: Option<&HeaderMap>, _stream_duration: Duration, _span: &Span| {
                //                 // ...
                //             },
                //         )
                .on_failure(
                    |_error: ServerErrorsFailureClass, _latency: Duration, _span: &Span| {
                        tracing::error!("Something went wrong")
                    },
                ),
        )
        // responseにSecureRemoteIpを伝搬させる
        // .layer(remote_ip::PropagateRemoteIpLayer::new(ip_source))
        // responseにx-request-idを伝搬させる
        .layer(tower_http::request_id::PropagateRequestIdLayer::new(
            x_request_id_name,
        ));

    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root))
        .route("/error_403", get(error_403))
        .route("/error_in_fn", get(error_in_fn))
        .route("/slow", get(|| tokio::time::sleep(Duration::from_secs(5))))
        .route("/forever", get(std::future::pending::<()>))
        .layer(service_layer);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    Ok(())
}

// #[tracing::instrument(name = "access")]
async fn root() -> Html<&'static str> {
    // async fn root(SecureClientIp(ip): SecureClientIp) -> Html<&'static str> {
    info!("root()");
    Html(
        r#"
    <html><body>
        <ul>
            <li><a href='/error_403'>error 403</a></li>
            <li><a href='/error_in_fn'>error in fn</a></li>
            <li><a href='/slow'>slow</a></li>
            <li><a href='/forever'>forever</a></li>
        </ul>
    "#,
    )
}

async fn error_403() -> impl IntoResponse {
    error!("error_403()");
    (http::StatusCode::SERVICE_UNAVAILABLE, "error_403_forbidden")
}

async fn error_in_fn() -> Result<(), ServerError> {
    error!("error_in_fn()");
    try_thing()?;
    Ok(())
}

fn try_thing() -> Result<(), anyhow::Error> {
    anyhow::bail!("try_thing() by anyhow")
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

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
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

// Make our own error that wraps `anyhow::Error`.
struct ServerError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        (
            http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for ServerError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

fn version_print(output_as_json: bool) -> anyhow::Result<()> {
    use std::collections::BTreeMap;

    if output_as_json {
        let build_envs = vergen_pretty::vergen_pretty_env!()
            .into_iter()
            .filter_map(|(k, v)| v.map(|v| (k, v)))
            .collect::<BTreeMap<_, _>>();
        let s = serde_json::to_string_pretty(&build_envs)?;
        println!("{}", s);
    } else {
        let stdout = std::io::stdout();
        let mut stdout = stdout.lock();

        let _pp = vergen_pretty::PrettyBuilder::default()
            .env(vergen_pretty::vergen_pretty_env!())
            .build()?
            .display(&mut stdout)?;
    }

    Ok(())
}
