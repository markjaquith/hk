use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::{
    git::Git,
    step::{RunType, Step},
    step_scheduler::StepScheduler,
    version, Result,
};
use miette::{bail, IntoDiagnostic};

impl Config {
    pub fn get() -> Result<Self> {
        let paths = vec![
            Path::new("hk.pkl"),
            Path::new("hk.toml"),
            Path::new("hk.yaml"),
            Path::new("hk.json"),
        ];
        for path in paths {
            if path.exists() {
                return Self::read(path);
            }
        }
        debug!("No config file found, using default");
        Ok(Self::default())
    }

    pub fn read(path: &Path) -> Result<Self> {
        let ext = path.extension().unwrap_or_default().to_str().unwrap();
        let mut config: Config = match ext {
            "toml" => {
                let raw = xx::file::read_to_string(path)?;
                toml::from_str(&raw).into_diagnostic()?
            }
            "yaml" => {
                let raw = xx::file::read_to_string(path)?;
                serde_yaml::from_str(&raw).into_diagnostic()?
            }
            "json" => {
                let raw = xx::file::read_to_string(path)?;
                serde_json::from_str(&raw).into_diagnostic()?
            }
            "pkl" => rpkl_jdx::from_config(path).into_diagnostic()?,
            _ => {
                bail!("Unsupported file extension: {}", ext);
            }
        };
        if let Some(min_hk_version) = &config.min_hk_version {
            version::version_cmp_or_bail(min_hk_version)?;
        }
        if let Some(pre_commit) = &mut config.pre_commit {
            for (name, step) in pre_commit.iter_mut() {
                step.name = name.clone();
            }
        }
        Ok(config)
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub min_hk_version: Option<String>,
    #[serde(rename = "pre-commit")]
    pub pre_commit: Option<IndexMap<String, Step>>,
    #[serde(rename = "pre-push")]
    pub pre_push: Option<IndexMap<String, Step>>,
}

impl std::fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", toml::to_string(self).unwrap())
    }
}

impl Config {
    pub async fn run_hook(&self, hook: &str, run_type: Option<RunType>, repo: &Git) -> Result<()> {
        let hook = match hook {
            "pre-commit" => &self.pre_commit,
            "pre-push" => &self.pre_push,
            _ => bail!("Invalid hook: {}", hook),
        }
        .clone()
        .unwrap_or_default();
        let all_files = matches!(run_type, Some(RunType::RunAll) | Some(RunType::FixAll));
        let mut runner = StepScheduler::new(&hook).with_all_files(all_files);
        if all_files {
            let all_files = repo.all_files()?;
            runner = runner.with_all_files(true).with_files(all_files);
        } else {
            let staged_files = repo.staged_files()?;
            runner = runner.with_files(staged_files);
        }
        runner.run().await?;
        info!("pre-commit done");
        Ok(())
    }
}
