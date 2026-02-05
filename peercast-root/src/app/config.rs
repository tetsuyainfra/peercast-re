use anyhow::Ok;

use crate::app::cli;

pub struct Config {}

pub fn create_config(args: &cli::Args) -> anyhow::Result<Config> {
    Ok(Config {})
}
