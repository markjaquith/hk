use crate::Result;
use crate::cli::Cli;
use clap::CommandFactory;

/// Generates a usage spec for the CLI
///
/// https://usage.jdx.dev
#[derive(Debug, clap::Args)]
#[clap(hide = true, verbatim_doc_comment)]
pub struct Usage {}

impl Usage {
    pub async fn run(&self) -> Result<()> {
        let mut cmd = Cli::command();
        let mut buf = vec![];
        clap_usage::generate(&mut cmd, "hk", &mut buf);
        let usage = String::from_utf8(buf).unwrap() + "\n" + include_str!("../hk-extras.usage.kdl");
        println!("{}", usage.trim());
        Ok(())
    }
}
