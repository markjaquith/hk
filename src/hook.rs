use clx::progress::{ProgressJob, ProgressJobBuilder, ProgressStatus};
use indexmap::IndexMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    signal,
    sync::{Mutex, OnceCell, RwLock, Semaphore},
};
use tokio_util::sync::CancellationToken;

use crate::{
    Result, env,
    git::{Git, GitStatus, StashMethod},
    glob,
    hook_options::HookOptions,
    settings::Settings,
    step::{CheckType, RunType, Step},
    step_scheduler::StepScheduler,
    ui::style,
    version,
};

#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(debug_assertions, serde(deny_unknown_fields))]
pub struct Hook {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub steps: IndexMap<String, Step>,
    #[serde(default)]
    pub fix: bool,
    pub stash: Option<StashMethod>,
}

pub struct HookContext {
    pub file_locks: Mutex<BTreeMap<PathBuf, Arc<RwLock<()>>>>,
    pub tctx: crate::tera::Context,
    // pub files_added: Mutex<usize>,
    pub run_type: RunType,
    pub semaphore: Arc<Semaphore>,
    pub failed: CancellationToken,
}

impl HookContext {
    pub fn new<P: AsRef<Path>>(
        files: impl IntoIterator<Item = P>,
        tctx: crate::tera::Context,
        run_type: RunType,
    ) -> Self {
        Self {
            file_locks: Mutex::new(
                files
                    .into_iter()
                    .map(|f| (f.as_ref().to_path_buf(), Arc::new(RwLock::new(()))))
                    .collect(),
            ),
            tctx,
            // files_added: Mutex::new(0),
            run_type,
            semaphore: Arc::new(Semaphore::new(Settings::get().jobs().get())),
            failed: CancellationToken::new(),
        }
    }
}

impl Hook {
    pub fn init(&mut self, hook_name: &str) {
        self.name = hook_name.to_string();
        for (name, step) in self.steps.iter_mut() {
            step.init(name);
        }
    }

    pub async fn run(&self, opts: HookOptions) -> Result<()> {
        if env::HK_SKIP_HOOK.contains(&self.name) {
            warn!("{}: skipping hook due to HK_SKIP_HOOK", &self.name);
            return Ok(());
        }
        let run_type = if *env::HK_FIX && self.fix {
            RunType::Fix
        } else {
            RunType::Check(CheckType::Check)
        };
        let hk_progress = self.start_hk_progress(run_type);
        if opts.to_ref.is_some() {
            // TODO: implement to_ref
        }
        let repo = Arc::new(Mutex::new(Git::new()?));
        let git_status = OnceCell::new();
        let stash_method = env::HK_STASH.or(self.stash).unwrap_or(StashMethod::None);
        watch_for_ctrl_c(repo.clone());
        let file_progress = ProgressJobBuilder::new().body(vec![
            "{{spinner()}} files - {{message}}{% if files is defined %} ({{files}} file{{files|pluralize}}){% endif %}".to_string(),
        ])
        .prop("message", "Fetching git status")
        .start();
        let files = self
            .file_list(
                &opts,
                repo.clone(),
                &git_status,
                stash_method,
                &file_progress,
            )
            .await?;

        if stash_method != StashMethod::None {
            let git_status = git_status
                .get_or_try_init(async || repo.lock().await.status())
                .await?;
            repo.lock()
                .await
                .stash_unstaged(&file_progress, stash_method, git_status)?;
        }

        let hook_ctx = Arc::new(HookContext::new(files.iter(), opts.tctx, run_type));
        let mut result = StepScheduler::new(self, hook_ctx, repo.clone())
            .with_linters(&opts.step)
            .run()
            .await;
        hk_progress.set_status(ProgressStatus::Done);

        if let Err(err) = repo.lock().await.pop_stash() {
            if result.is_ok() {
                result = Err(err);
            } else {
                warn!("Failed to pop stash: {}", err);
            }
        }
        result
    }

    async fn file_list(
        &self,
        opts: &HookOptions,
        repo: Arc<Mutex<Git>>,
        git_status: &OnceCell<GitStatus>,
        stash_method: StashMethod,
        file_progress: &ProgressJob,
    ) -> Result<BTreeSet<PathBuf>> {
        let mut files = if let Some(files) = &opts.files {
            files
                .iter()
                .map(|f| {
                    let p = PathBuf::from(f);
                    if p.is_dir() {
                        all_files_in_dir(&p)
                    } else {
                        Ok(vec![p])
                    }
                })
                .flatten_ok()
                .collect::<Result<BTreeSet<_>>>()?
        } else if let Some(glob) = &opts.glob {
            file_progress.prop("message", "Fetching files matching glob");
            // TODO: should fetch just the files that match the glob
            let all_files = repo.lock().await.all_files()?;
            glob::get_matches(glob, &all_files)?.into_iter().collect()
        } else if let (Some(from), Some(to)) = (&opts.from_ref, &opts.to_ref) {
            file_progress.prop(
                "message",
                &format!("Fetching files between {} and {}", from, to),
            );
            repo.lock()
                .await
                .files_between_refs(from, to)?
                .into_iter()
                .collect()
        } else if opts.all {
            file_progress.prop("message", "Fetching all files in repo");
            repo.lock().await.all_files()?.into_iter().collect()
        } else if stash_method != StashMethod::None {
            file_progress.prop("message", "Fetching staged files");
            let git_status = git_status
                .get_or_try_init(async || repo.lock().await.status())
                .await?;
            git_status.staged_files.iter().cloned().collect()
        } else {
            file_progress.prop("message", "Fetching modified files");
            let git_status = git_status
                .get_or_try_init(async || repo.lock().await.status())
                .await?;
            git_status
                .staged_files
                .iter()
                .chain(git_status.unstaged_files.iter())
                .cloned()
                .collect()
        };
        for exclude in opts.exclude.as_ref().unwrap_or(&vec![]) {
            let exclude = Path::new(&exclude);
            files.retain(|f| !f.starts_with(exclude));
        }
        if let Some(exclude_glob) = &opts.exclude_glob {
            let f = files.iter().collect::<Vec<_>>();
            let exclude_files = glob::get_matches(exclude_glob, &f)?
                .into_iter()
                .collect::<HashSet<_>>();
            files.retain(|f| !exclude_files.contains(f));
        }
        file_progress.prop("files", &files.len());
        file_progress.set_status(ProgressStatus::Done);
        Ok(files)
    }

    fn start_hk_progress(&self, run_type: RunType) -> Arc<ProgressJob> {
        let mut hk_progress = ProgressJobBuilder::new()
            .body(vec!["{{hk}}{{hook}}{{message}}".to_string()])
            .prop(
                "hk",
                &format!(
                    "{} {} {}",
                    style::emagenta("hk").bold(),
                    style::edim(version::version()),
                    style::edim("by @jdx")
                )
                .to_string(),
            );
        if self.name == "check" || self.name == "fix" {
            hk_progress = hk_progress.prop("hook", "");
        } else {
            hk_progress = hk_progress.prop(
                "hook",
                &style::edim(format!(" – {}", self.name)).to_string(),
            );
        }
        if run_type == RunType::Fix {
            hk_progress = hk_progress.prop("message", &style::edim(" – fix").to_string());
        } else {
            hk_progress = hk_progress.prop("message", &style::edim(" – check").to_string());
        }
        hk_progress.start()
    }
}

fn watch_for_ctrl_c(repo: Arc<Mutex<Git>>) {
    tokio::spawn(async move {
        if let Err(err) = signal::ctrl_c().await {
            warn!("Failed to watch for ctrl-c: {}", err);
        }
        if let Err(err) = repo.lock().await.pop_stash() {
            warn!("Failed to pop stash: {}", err);
        }
        clx::progress::flush();
        // TODO: gracefully stop child processes
        std::process::exit(1);
    });
}

fn all_files_in_dir(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = vec![];
    for entry in xx::file::ls(dir)? {
        if entry.is_dir() {
            files.extend(all_files_in_dir(&entry)?);
        } else {
            files.push(entry);
        }
    }
    Ok(files)
}
