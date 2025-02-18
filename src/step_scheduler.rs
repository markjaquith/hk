use indexmap::IndexMap;
use itertools::Itertools;
use miette::IntoDiagnostic;
use std::{collections::HashSet, path::PathBuf, sync::Arc};
use tokio::sync::{Mutex, RwLock, Semaphore};
use tokio::task::JoinSet;

use crate::{
    env, glob,
    settings::Settings,
    step::{RunType, Step},
};
use crate::{step::StepContext, Result};

#[derive(Debug)]
pub struct StepScheduler {
    run_type: RunType,
    steps: Vec<Step>,
    files: Vec<PathBuf>,
    failed: Arc<Mutex<bool>>,
    semaphore: Arc<Semaphore>,
    all_files: bool,
    file_locks: Mutex<IndexMap<PathBuf, Arc<RwLock<()>>>>,
}

impl StepScheduler {
    pub fn new(hook: &IndexMap<String, Step>, run_type: RunType) -> Self {
        let settings = Settings::get();
        Self {
            run_type,
            steps: hook.values().cloned().collect(),
            files: vec![],
            failed: Arc::new(Mutex::new(false)),
            semaphore: Arc::new(Semaphore::new(settings.jobs().get())),
            all_files: true,
            file_locks: Default::default(),
        }
    }

    pub fn with_all_files(mut self, all_files: bool) -> Self {
        self.all_files = all_files;
        self
    }

    pub fn with_files(mut self, files: Vec<PathBuf>) -> Self {
        self.files = files;
        self
    }

    /// Returns a subset of steps that have a fix command and need to at least 1 file another step will need a read/write lock on
    fn fix_steps_in_contention<'a>(
        &self,
        steps: &[&'a Step],
        files: &[PathBuf],
    ) -> Result<HashSet<&'a Step>> {
        if matches!(self.run_type, RunType::CheckAll | RunType::Check) {
            return Ok(Default::default());
        }
        let files_per_step: IndexMap<&Step, HashSet<PathBuf>> = steps
            .iter()
            .map(|step| {
                let files = glob::get_matches(step.glob.as_ref().unwrap_or(&vec![]), files)?;
                Ok((*step, files.into_iter().collect()))
            })
            .collect::<Result<_>>()?;
        let fix_steps = steps
            .iter()
            .copied()
            .filter(|step| {
                matches!(
                    step.available_run_type(self.run_type),
                    Some(RunType::Fix) | Some(RunType::FixAll)
                )
            })
            .filter(|step| {
                let other_files = files_per_step
                    .iter()
                    .filter(|(k, _)| *k != step)
                    .map(|(_, v)| v)
                    .collect::<Vec<_>>();
                files_per_step.get(*step).unwrap().iter().any(|file| {
                    other_files
                        .iter()
                        .any(|other_files| other_files.contains(file))
                })
            })
            .collect();
        Ok(fix_steps)
    }

    async fn run_step(
        &self,
        step: &Step,
        set: &mut JoinSet<Result<()>>,
        ctx: StepContext,
    ) -> Result<()> {
        let settings = Settings::get();
        let semaphore = self.semaphore.clone();
        let permit = semaphore.clone().acquire_owned().await.into_diagnostic()?;
        let failed = self.failed.clone();
        if *failed.lock().await {
            trace!("{step}: skipping step due to previous failure");
            return Ok(());
        }
        // Check if step should be skipped based on HK_SKIP_STEPS
        if crate::env::HK_SKIP_STEPS.contains(&step.name) {
            warn!("{step}: skipping step due to HK_SKIP_STEPS");
            return Ok(());
        }
        if let Some(profiles) = step.enabled_profiles() {
            let enabled_profiles = settings.enabled_profiles();
            let missing_profiles = profiles.difference(&enabled_profiles).collect::<Vec<_>>();
            if !missing_profiles.is_empty() {
                trace!(
                    "{step}: skipping step due to missing profile: {}",
                    missing_profiles.iter().join(", ")
                );
                return Ok(());
            }
        }
        if let Some(profiles) = step.disabled_profiles() {
            let enabled_profiles = settings.enabled_profiles();
            let disabled_profiles = profiles.intersection(&enabled_profiles).collect::<Vec<_>>();
            if !disabled_profiles.is_empty() {
                trace!(
                    "{step}: skipping step due to disabled profile: {}",
                    disabled_profiles.iter().join(", ")
                );
                return Ok(());
            }
        }

        trace!("{step}: spawning step");
        let step = step.clone();
        set.spawn(async move {
            let mut _permit = Some(permit);
            let flocks = ctx.file_locks.values().cloned().collect::<Vec<_>>();
            let mut read_flocks = vec![];
            let mut write_flocks = vec![];
            for lock in flocks.iter() {
                match (step.stomp, ctx.run_type) {
                    (true, _) | (_, RunType::CheckAll | RunType::Check) => match lock.try_read() {
                        Ok(lock) => read_flocks.push(lock),
                        Err(_) => {
                            _permit = None;
                            read_flocks.push(lock.read().await);
                        }
                    },
                    (_, RunType::FixAll | RunType::Fix) => match lock.try_write() {
                        Ok(lock) => write_flocks.push(lock),
                        Err(_) => {
                            _permit = None;
                            write_flocks.push(lock.write().await);
                        }
                    },
                }
            }
            if _permit.is_some() {
                _permit = Some(semaphore.acquire_owned().await.into_diagnostic()?);
            }
            match step.run(ctx).await {
                Ok(()) => Ok(()),
                Err(e) => {
                    // Mark as failed to prevent new steps from starting
                    *failed.lock().await = true;
                    Err(e.wrap_err(step.name))
                }
            }
        });
        Ok(())
    }

    pub async fn run(self) -> Result<()> {
        let runner = Arc::new(self);
        // groups is a list of list of steps which are separated by exclusive steps
        // any exclusive step will be in a group by itself
        let groups = runner.steps.iter().fold(vec![], |mut groups, step| {
            if step.exclusive || groups.is_empty() {
                groups.push(vec![]);
            }
            groups.last_mut().unwrap().push(step);
            groups
        });

        for group in groups {
            let mut set = JoinSet::new();
            let fix_steps_in_contention = runner.fix_steps_in_contention(&group, &runner.files)?;

            // Spawn all tasks
            for step in &group {
                let Some(run_type) = step.available_run_type(runner.run_type) else {
                    debug!("{step}: skipping step due to no available run type");
                    continue;
                };
                let files = if let Some(glob) = &step.glob {
                    let matches = glob::get_matches(glob, &runner.files)?;
                    if matches.is_empty() {
                        debug!("{step}: no matches for step");
                        continue;
                    }
                    matches
                } else {
                    runner.files.clone()
                };
                let ctx = StepContext {
                    run_type,
                    file_locks: runner
                        .file_locks
                        .lock()
                        .await
                        .iter()
                        .filter(|(k, _)| files.contains(k))
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect(),
                    files,
                };

                if *env::HK_CHECK_FIRST
                    && step.check_first
                    && matches!(ctx.run_type, RunType::FixAll | RunType::Fix)
                    && fix_steps_in_contention.contains(step)
                {
                    let mut ctx = ctx.clone();
                    ctx.run_type = match ctx.run_type {
                        RunType::FixAll => RunType::CheckAll,
                        RunType::Fix => RunType::Check,
                        _ => unreachable!(),
                    };
                    debug!("{step}: running check step first due to fix step contention");
                    if let Err(e) = runner.run_step(step, &mut set, ctx).await {
                        warn!("{step}: failed check step first: {e}");
                    } else {
                        debug!("{step}: successfully ran check step first");
                        continue;
                    }
                }
                runner.run_step(step, &mut set, ctx).await?;
            }

            // Wait for tasks and abort on first error
            while let Some(result) = set.join_next().await {
                match result {
                    Ok(Ok(_)) => continue, // Step completed successfully
                    Ok(Err(e)) => {
                        // Task failed to execute
                        set.abort_all();
                        return Err(e);
                    }
                    Err(e) => {
                        // JoinError occurred
                        set.abort_all();
                        return Err(e).into_diagnostic();
                    }
                }
            }
        }
        Ok(())
    }
}
