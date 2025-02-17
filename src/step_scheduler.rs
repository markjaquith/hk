use indexmap::IndexMap;
use miette::IntoDiagnostic;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::{Mutex, Semaphore};
use tokio::task::JoinSet;

use crate::{settings::Settings, step::Step};
use crate::{step::StepContext, Result};

#[derive(Debug)]
pub struct StepScheduler {
    steps: Vec<Step>,
    files: Vec<PathBuf>,
    failed: Arc<Mutex<bool>>,
    semaphore: Arc<Semaphore>,
    all_files: bool,
    jobs: u32,
}

impl StepScheduler {
    pub fn new(hook: &IndexMap<String, Step>) -> Self {
        let settings = Settings::get();
        Self {
            steps: hook.values().cloned().collect(),
            files: vec![],
            failed: Arc::new(Mutex::new(false)),
            semaphore: Arc::new(Semaphore::new(settings.jobs().get())),
            jobs: settings.jobs().get() as u32,
            all_files: true,
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

    async fn run_step(
        &self,
        step: &Step,
        set: &mut JoinSet<Result<()>>,
        ctx: Arc<StepContext>,
    ) -> Result<()> {
        let semaphore = self.semaphore.clone();
        let permits = if step.exclusive {
            // Get all the locks because it's an exclusive step so it starts after all previous steps have finished
            semaphore
                .acquire_many_owned(self.jobs)
                .await
                .into_diagnostic()?
        } else {
            // Get a lock on the semaphore so we only run max jobs at a time
            semaphore.acquire_owned().await.into_diagnostic()?
        };
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
        debug!("spawning step: {:?}", step.name);
        let step = step.clone();
        set.spawn(async move {
            let _permits = permits;
            match step.run(&ctx).await {
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
        let mut set = JoinSet::new();
        let ctx = Arc::new(StepContext {
            all_files: runner.all_files,
            files: runner.files.clone(),
        });

        // Spawn all tasks
        for step in &runner.steps {
            runner.run_step(step, &mut set, ctx.clone()).await?;
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

        Ok(())
    }
}
