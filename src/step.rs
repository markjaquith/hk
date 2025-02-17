use crate::Result;
use crate::{env, tera};
use crate::{git::Git, glob};
use ensembler::CmdLineRunner;
use itertools::Itertools;
use miette::IntoDiagnostic;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{fmt, sync::Arc};

use serde_with::{serde_as, OneOrMany};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub struct FileLocks {
    pub read: Option<String>,
    pub write: Option<String>,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub r#type: Option<String>,
    #[serde(default)]
    pub name: String,
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub glob: Option<Vec<String>>,
    pub check: Option<String>,
    pub fix: Option<String>,
    pub check_all: Option<String>,
    pub fix_all: Option<String>,
    pub root: Option<PathBuf>,
    pub exclusive: bool,
    pub file_locks: Option<FileLocks>,
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub stage: Option<Vec<String>>,
}

impl fmt::Display for Step {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileKind {
    Text,
    Binary,
    Executable,
    NotExecutable,
    Symlink,
    NotSymlink,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RunType {
    Check,
    Fix,
    CheckAll,
    FixAll,
}

impl Step {
    pub async fn run(&self, ctx: &StepContext) -> Result<()> {
        let mut tctx = tera::Context::default();
        let staged_files = if let Some(glob) = &self.glob {
            let matches = glob::get_matches(glob, &ctx.files)?;
            if matches.is_empty() {
                debug!("{self}: no matches for step");
                return Ok(());
            }
            matches
        } else {
            ctx.files.clone()
        };
        tctx.with_globs(self.glob.as_ref().unwrap_or(&vec![]));
        tctx.with_files(staged_files.as_ref());
        let pr = self.build_pr();
        let run_type = if *env::HK_FIX && self.fix_all.is_some() || self.fix.is_some() {
            if self.fix.is_none() || (ctx.all_files && self.fix_all.is_some()) {
                RunType::FixAll
            } else {
                RunType::Fix
            }
        } else if self.check.is_none() || (ctx.all_files && self.check_all.is_some()) {
            RunType::CheckAll
        } else {
            RunType::Check
        };
        let Some(run) = (match run_type {
            RunType::Check => self.check.as_ref(),
            RunType::Fix => self.fix.as_ref(),
            RunType::CheckAll => self.check_all.as_ref(),
            RunType::FixAll => self.fix_all.as_ref(),
        }) else {
            warn!("{}: no run command", self);
            return Ok(());
        };
        let run = tera::render(run, &tctx).unwrap();
        pr.set_message(run.clone());
        CmdLineRunner::new("sh")
            .arg("-c")
            .arg(run)
            .with_pr(pr.clone())
            .execute()
            .await
            .into_diagnostic()?;
        let pathspecs_to_add = if matches!(run_type, RunType::Fix | RunType::FixAll) {
            let pathspecs_to_add = if let Some(stage) = &self.stage {
                let stage = stage
                    .iter()
                    .map(|s| tera::render(s, &tctx).unwrap())
                    .collect_vec();
                glob::get_matches(&stage, &staged_files)?
            } else if self.glob.is_some() {
                staged_files
            } else {
                vec![]
            }
            .iter()
            .map(|f| f.to_string_lossy().to_string())
            .collect_vec();
            if !pathspecs_to_add.is_empty() {
                pr.set_message(format!("staging {}", pathspecs_to_add.join(" ")));
                let mut repo = Git::new()?;
                if !pathspecs_to_add.is_empty() {
                    if let Err(err) =
                        repo.add(&pathspecs_to_add.iter().map(|f| f.as_str()).collect_vec())
                    {
                        warn!("{self}: failed to add files to index: {err}");
                    }
                }
            }
            pathspecs_to_add
        } else {
            vec![]
        };

        if run_type == RunType::CheckAll
            || run_type == RunType::FixAll
            || pathspecs_to_add.is_empty()
        {
            pr.finish_with_message("done".to_string());
        } else {
            pr.finish_with_message(format!(
                "{} file{}",
                pathspecs_to_add.len(),
                if pathspecs_to_add.len() == 1 { "" } else { "s" }
            ));
        }
        Ok(())
    }

    fn build_pr(&self) -> Arc<Box<dyn clx::SingleReport>> {
        let mpr = clx::MultiProgressReport::get();
        mpr.add(&self.name)
    }
}

pub struct StepContext {
    pub all_files: bool,
    pub files: Vec<PathBuf>,
}
