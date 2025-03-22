use std::path::PathBuf;
use std::sync::LazyLock;

use indexmap::IndexMap;

use crate::{Result, git::Git, tera::Context};
use crate::{config::Config, step::CheckType};
use crate::{
    env,
    step::{RunType, Step},
};

#[derive(Debug, clap::Args)]
#[clap(visible_alias = "cm")]
pub struct CommitMsg {
    /// The path to the file that contains the commit message
    commit_msg_file: PathBuf,
}

impl CommitMsg {
    pub async fn run(&self) -> Result<()> {
        let config = Config::get()?;
        if env::HK_SKIP_HOOK.contains("commit-msg") {
            warn!("commit-msg: skipping hook due to HK_SKIP_HOOK");
            return Ok(());
        }
        let repo = Git::new()?;
        static HOOK: LazyLock<IndexMap<String, Step>> = LazyLock::new(Default::default);
        let hook = config.hooks.get("commit-msg").unwrap_or(&HOOK);
        let mut tctx = Context::default();
        tctx.insert("commit_msg_file", &self.commit_msg_file.to_string_lossy());
        config
            .run_hook(
                false,
                hook,
                RunType::Check(CheckType::Check),
                &repo,
                &[],
                tctx,
                None,
                None,
            )
            .await?;
        Ok(())
    }
}
