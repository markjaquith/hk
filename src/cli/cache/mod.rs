use crate::Result;

mod clear;

/// Manage hk internal cache
#[derive(Debug, clap::Args)]
#[clap(hide = true)] // TODO: unhide if we actually use cache (which we probably will)
pub struct Cache {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, clap::Subcommand)]
enum Commands {
    /// Clear the cache directory
    Clear(clear::Clear),
}

impl Cache {
    pub async fn run(self) -> Result<()> {
        match self.command {
            Commands::Clear(cmd) => cmd.run().await,
        }
    }
}
