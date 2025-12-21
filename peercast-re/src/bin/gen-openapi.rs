use clap::Parser;
use peercast_re::{handler::api};
use utoipa::OpenApi;


#[derive(Clone, Debug, Parser)]
#[clap(
        name = env!("CARGO_PKG_NAME"),
        author = env!("CARGO_PKG_AUTHORS"),
        about = env!("CARGO_PKG_DESCRIPTION"),
    )]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Clone, Debug, clap::Subcommand)]
pub enum Commands {
    V1,
    V2,
}

fn main() {

    let args = Args::parse();
    let api =     match args.command {
        None => {
            // eprintln!("Please specify a command: v1 or v2");
            // std::process::exit(1);
            api::ApiSetV1::openapi()
        }
        Some(Commands::V1) => api::ApiSetV1::openapi(),
        Some(Commands::V2) => api::ApiSetV2::openapi(),
    };

    // let api =api::ApiRoot::openapi();
    let openapi_json = api.to_pretty_json().unwrap();

    println!("{}", openapi_json);
}
