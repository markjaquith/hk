use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use miette::{miette, IntoDiagnostic};
use std::{collections::{HashMap, HashSet}, iter::once, path::{Path, PathBuf}, sync::Arc};
use tokio::sync::{
    Mutex, OwnedRwLockReadGuard, OwnedRwLockWriteGuard, OwnedSemaphorePermit, RwLock, Semaphore,
};
use tokio::task::JoinSet;

use crate::{
    config::Config,
    env,
    git::Git,
    glob,
    settings::Settings,
    step::{RunType, Step},
};
use crate::{step::StepContext, Result};

pub struct StepScheduler<'a> {
    run_type: RunType,
    repo: &'a Git,
    steps: Vec<Step>,
    files: Vec<PathBuf>,
    failed: Arc<Mutex<bool>>,
    semaphore: Arc<Semaphore>,
    file_locks: Mutex<IndexMap<PathBuf, Arc<RwLock<()>>>>,
}

impl<'a> StepScheduler<'a> {
    pub fn new(hook: &IndexMap<String, Step>, run_type: RunType, repo: &'a Git) -> Self {
        let settings = Settings::get();
        let config = Config::get().unwrap();
        Self {
            run_type,
            repo,
            steps: hook
                .values()
                .flat_map(|s| match &s.r#type {
                    Some(r#type) if r#type == "check" || r#type == "fix" => config
                        .linters
                        .values()
                        .cloned()
                        .map(|linter| linter.into())
                        .collect(),
                    _ => vec![s.clone()],
                })
                .collect(),
            files: vec![],
            failed: Arc::new(Mutex::new(false)),
            semaphore: Arc::new(Semaphore::new(settings.jobs().get())),
            file_locks: Default::default(),
        }
    }

    pub fn with_files(mut self, files: Vec<PathBuf>) -> Self {
        self.files = files;
        self
    }

    pub async fn run(self) -> Result<()> {
        *self.file_locks.lock().await = self
            .files
            .iter()
            .map(|file| (file.clone(), Arc::new(RwLock::new(()))))
            .collect();
        // groups is a list of list of steps which are separated by exclusive steps
        // any exclusive step will be in a group by itself
        let groups = self.steps.iter().fold(vec![], |mut groups, step| {
            if step.exclusive || groups.is_empty() {
                groups.push(vec![]);
            }
            groups.last_mut().unwrap().push(step);
            groups
        });

        for group in groups {
            let mut set = JoinSet::new();
            let depend_locks: IndexMap<String, _> = group
                .iter()
                .map(|s| (s.name.clone(), Arc::new(RwLock::new(()))))
                .collect();
            let files_in_contention = Arc::new(self.files_in_contention(&group, &self.files)?);

            // Spawn all tasks
            for step in &group {
                let Some(run_type) = step.available_run_type(self.run_type) else {
                    debug!("{step}: skipping step due to no available run type");
                    continue;
                };
                let files = if let Some(glob) = &step.glob {
                    let matches = glob::get_matches(glob, &self.files)?;
                    if matches.is_empty() {
                        debug!("{step}: no matches for step");
                        continue;
                    }
                    matches
                } else {
                    self.files.clone()
                };
                let ctx = StepContext {
                    run_type,
                    files_to_add: vec![],
                    file_locks: self
                        .file_locks
                        .lock()
                        .await
                        .iter()
                        .filter(|(k, _)| files.contains(k))
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect(),
                    files,
                };

                let depends = step
                    .depends
                    .iter()
                    .chain(once(&step.name))
                    .map(|dep| {
                        depend_locks
                            .iter()
                            .find(|(name, _)| *name == dep || **name == step.name)
                            .map(|(name, lock)| (name.to_string(), lock.clone()))
                            .ok_or_else(|| miette!("No step named {dep} found in group"))
                    })
                    .collect::<Result<IndexMap<String, _>>>()?;

                if !step.is_profile_enabled() {
                    continue;
                }
                
                self.run_step(
                    depends,
                    step,
                    &mut set,
                    ctx,
                    files_in_contention.clone(),
                )
                .await?;
            }

            // Wait for tasks and abort on first error
            let mut files_to_stage = IndexSet::new();
            while let Some(result) = set.join_next().await {
                match result {
                    Ok(Ok(ctx)) => {
                        files_to_stage.extend(ctx.files_to_add);
                    }
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

            if !files_to_stage.is_empty() {
                trace!("staging files: {:?}", &files_to_stage);
                self.repo.add(
                    &files_to_stage
                        .iter()
                        .map(|f| f.to_str().unwrap())
                        .collect_vec(),
                )?;
            }
        }
        Ok(())
    }

    fn files_in_contention(&self, steps: &[&Step], files: &[PathBuf]) -> Result<HashSet<PathBuf>> {
        let files_by_step: HashMap<&Step, Vec<PathBuf>> = steps
            .iter()
            .map(|step| {
                let files = glob::get_matches(step.glob.as_ref().unwrap_or(&vec![]), files)?;
                Ok((*step, files))
            })
            .collect::<Result<_>>()?;
        let mut steps_per_file: HashMap<&Path, Vec<&Step>> = Default::default();
        for (step, files) in files_by_step.iter() {
            for file in files {
                steps_per_file.entry(file.as_path()).or_default().push(step);
            }
        }

        let mut files_in_contention = HashSet::new();
        for (file, steps) in steps_per_file.iter() {
            if steps.iter().any(|step| step.available_run_type(self.run_type) == Some(RunType::Fix)) {
                files_in_contention.insert(file.to_path_buf());
            }
        }

        Ok(files_in_contention)
    }

    async fn run_step(
        &self,
        mut depends: IndexMap<String, Arc<RwLock<()>>>,
        step: &Step,
        set: &mut JoinSet<Result<StepContext>>,
        ctx: StepContext,
        files_in_contention: Arc<HashSet<PathBuf>>,
    ) -> Result<()> {
        let semaphore = self.semaphore.clone();
        let permit = semaphore.clone().acquire_owned().await.into_diagnostic()?;
        let failed = self.failed.clone();
        if *failed.lock().await {
            trace!("{step}: skipping step due to previous failure");
            return Ok(());
        }
        let depend_self = depends.shift_remove(&step.name).unwrap();
        let self_depend_write_lock = depend_self.write_owned().await;

        trace!("{step}: spawning step");
        let step = step.clone();
        set.spawn(async move {
            let _self_depend_write_lock = self_depend_write_lock;
            let depends = depends;
            if *env::HK_CHECK_FIRST
                && step.check_first
                && matches!(ctx.run_type, RunType::Fix)
                && ctx.files.iter().any(|file| files_in_contention.contains(file))
            {
                let mut check_ctx = ctx.clone();
                check_ctx.run_type = match ctx.run_type {
                    RunType::Fix => RunType::Check,
                    _ => unreachable!(),
                };
                debug!("{step}: running check step first due to fix step contention");
                match run(
                    check_ctx,
                    semaphore.clone(),
                    &step,
                    failed.clone(),
                    permit,
                    &depends,
                )
                .await
                {
                    Ok(ctx) => {
                        debug!("{step}: successfully ran check step first");
                        Ok(ctx)
                    }
                    Err(e) => {
                        warn!("{step}: failed check step first: {e}");
                        let permit = semaphore.clone().acquire_owned().await.into_diagnostic()?;
                        run(ctx, semaphore, &step, failed, permit, &depends).await
                    }
                }
            } else {
                run(ctx, semaphore, &step, failed, permit, &depends).await
            }
        });
        Ok(())
    }
}

async fn run(
    ctx: StepContext,
    semaphore: Arc<Semaphore>,
    step: &Step,
    failed: Arc<Mutex<bool>>,
    permit: OwnedSemaphorePermit,
    depends: &IndexMap<String, Arc<RwLock<()>>>,
) -> Result<StepContext> {
    let _locks = StepLocks::lock(step, &ctx, semaphore, permit, depends).await?;
    match step.run(ctx).await {
        Ok(ctx) => Ok(ctx),
        Err(err) => {
            // Mark as failed to prevent new steps from starting
            *failed.lock().await = true;
            Err(err.wrap_err(step.name.clone()))
        }
    }
}

#[allow(unused)]
struct StepLocks {
    read_flocks: Vec<OwnedRwLockReadGuard<()>>,
    write_flocks: Vec<OwnedRwLockWriteGuard<()>>,
    permit: OwnedSemaphorePermit,
}

impl StepLocks {
    fn try_lock(
        step: &Step,
        ctx: &StepContext,
        depends: &IndexMap<String, Arc<RwLock<()>>>,
        permit: OwnedSemaphorePermit,
    ) -> Option<Self> {
        let mut read_flocks = vec![];
        let mut write_flocks = vec![];
        for (name, depends) in depends.iter() {
            if depends.try_read().is_err() {
                trace!("{step}: waiting for {name} to finish");
                return None;
            }
        }
        for (path, lock) in ctx.file_locks.iter() {
            let lock = lock.clone();
            match (step.stomp, ctx.run_type) {
                (_, RunType::Run) => {}
                (true, _) | (_, RunType::Check) => match lock.clone().try_read_owned() {
                    Ok(lock) => read_flocks.push(lock),
                    Err(_) => {
                        trace!("{step}: waiting for {} to finish", path.display());
                        return None;
                    }
                },
                (_, RunType::Fix) => match lock.clone().try_write_owned() {
                    Ok(lock) => write_flocks.push(lock),
                    Err(_) => {
                        trace!("{step}: waiting for {} to finish", path.display());
                        return None;
                    }
                },
            }
        }
        Some(StepLocks {
            read_flocks,
            write_flocks,
            permit,
        })
    }

    async fn lock(
        step: &Step,
        ctx: &StepContext,
        semaphore: Arc<Semaphore>,
        permit: OwnedSemaphorePermit,
        depends: &IndexMap<String, Arc<RwLock<()>>>,
    ) -> Result<Self> {
        if let Some(locks) = Self::try_lock(step, ctx, depends, permit) {
            return Ok(locks);
        }
        let mut read_flocks = vec![];
        let mut write_flocks = vec![];
        for (_name, depends) in depends.iter() {
            read_flocks.push(depends.clone().read_owned().await);
        }
        for (_path, lock) in ctx.file_locks.iter() {
            let lock = lock.clone();
            match (step.stomp, ctx.run_type) {
                (_, RunType::Run) => {}
                (true, _) | (_, RunType::Check) => read_flocks.push(lock.clone().read_owned().await),
                (_, RunType::Fix) => write_flocks.push(lock.clone().write_owned().await),
            }
        }
        Ok(Self {
            read_flocks,
            write_flocks,
            permit: semaphore.clone().acquire_owned().await.into_diagnostic()?,
        })
    }
}
