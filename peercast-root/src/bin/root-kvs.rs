use clap::Parser;

fn main() {
    let args = Args::parse();

    match args.command {
        SubCommand::List => todo!(),
        SubCommand::Show {
            key,
        } => todo!(),
    }
}

const ABOUT: &str = r#"
peercast-rootのKVSを参照するためのプログラム
次のように使う
> $ root-kvs list
> $ root-kvs show <key>
"#;

#[derive(Debug, Clone, Parser)]
#[command(name = env!("CARGO_BIN_NAME"))]
#[command(version, long_about = ABOUT)]
pub struct Args {
    #[command(subcommand)]
    pub command: SubCommand,
}

#[derive(Debug, Clone, Parser)]
pub enum SubCommand {
    /// KVSの内容を一覧表示する
    List,
    /// 指定したkeyの内容を表示する
    Show {
        /// 取得するkey
        key: String,
    },
}
