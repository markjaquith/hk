use std::path::Path;

use crate::{config::Config as HKConfig, Result};

/// Generate a default hk.toml configuration file
#[derive(Debug, clap::Args)]
#[clap(visible_alias = "cfg")]
pub struct Config {}

impl Config {
    pub async fn run(&self) -> Result<()> {
        warn!("this output is almost certain to change in a future version");
        let cfg = HKConfig::read(Path::new("hk.pkl"))?;
        println!("{}", cfg);
        Ok(())
    }
}
