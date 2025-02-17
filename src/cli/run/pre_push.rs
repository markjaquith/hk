use crate::config::Config;
use crate::{env, step::RunType};
use crate::{git::Git, Result};

/// Sets up git hooks to run hk
#[derive(Debug, clap::Args)]
#[clap(visible_alias = "ph")]
pub struct PrePush {
    /// Run on all files instead of just staged files
    #[clap(short, long)]
    all: bool,
    /// Run fix command instead of run command
    /// This is the default behavior unless HK_FIX=0
    #[clap(short, long)]
    fix: bool,
    /// Run run command instead of fix command
    #[clap(short, long)]
    run: bool,
    /// Remote name
    remote: String,
    /// Remote URL
    url: String,
}

#[allow(unreachable_code)]
impl PrePush {
    pub async fn run(&self) -> Result<()> {
        unimplemented!(
            "pre-push is not yet implemented. We need support for --from-ref and --to-ref"
        );
        let config = Config::get()?;
        let mut repo = Git::new()?;
        let run_type = if self.all {
            if self.fix || *env::HK_FIX {
                Some(RunType::FixAll)
            } else {
                Some(RunType::RunAll)
            }
        } else if self.fix || *env::HK_FIX {
            Some(RunType::Fix)
        } else {
            Some(RunType::Run)
        };
        let mut result = config.run_hook("pre-push", run_type, &repo).await;

        if let Err(err) = repo.pop_stash() {
            if result.is_ok() {
                result = Err(err);
            } else {
                warn!("Failed to pop stash: {}", err);
            }
        }
        result
    }
}
