use std::path::PathBuf;

use crate::{config::Config, env, Result};

/// Sets up git hooks to run hk
#[derive(Debug, clap::Args)]
#[clap(visible_alias = "i")]
pub struct Install {
    /// Use `mise x` to execute hooks. With this, it won't
    /// be necessary to activate mise in order to run hooks
    /// with mise tools.
    ///
    /// Set HK_MISE=1 to make this default behavior.
    #[clap(long, verbatim_doc_comment)]
    mise: bool,
}

impl Install {
    pub async fn run(&self) -> Result<()> {
        let config = Config::get()?;
        let hooks = PathBuf::from(".git/hooks");
        let add_hook = |hook: &str| {
            let hook_file = hooks.join(hook);
            let command = if *env::HK_MISE || self.mise {
                format!("mise x -- hk run {hook}")
            } else {
                format!("hk run {hook}")
            };
            let hook_content = format!(
                r#"#!/bin/sh
{command} "$@"
"#
            );
            xx::file::write(&hook_file, &hook_content)?;
            xx::file::make_executable(&hook_file)?;
            println!("Installed hk hook: .git/hooks/{hook}");
            Result::<(), miette::Report>::Ok(())
        };
        if config.pre_commit.is_some() {
            add_hook("pre-commit")?;
        }
        if config.pre_push.is_some() {
            add_hook("pre-push")?;
        }
        Ok(())
    }
}
