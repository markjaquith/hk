use crate::version;
use crate::Result;

/// Print the version of hk
#[derive(Debug, clap::Args)]
pub struct Version {}

impl Version {
    pub async fn run(&self) -> Result<()> {
        println!("{}", version::version());
        Ok(())
    }
}
