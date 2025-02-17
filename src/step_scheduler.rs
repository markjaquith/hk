use indexmap::IndexMap;
use miette::IntoDiagnostic;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::{Mutex, RwLock, Semaphore};
use tokio::task::JoinSet;

use crate::{
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
    jobs: u32,
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
            jobs: settings.jobs().get() as u32,
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

    async fn run_step(&self, step: &Step, set: &mut JoinSet<Result<()>>) -> Result<()> {
        let Some(run_type) = step.available_run_type(self.run_type) else {
            debug!("{step}: skipping step due to no available run type");
            return Ok(());
        };
        let ctx = Arc::new(StepContext {
            run_type,
            all_files: self.all_files,
            files: self.files.clone(),
        });

        let permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .into_diagnostic()?;
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
        trace!("{step}: spawning step");
        let step = step.clone();
        set.spawn(async move {
            let _permit = permit;
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

            // Spawn all tasks
            for step in &group {
                runner.run_step(step, &mut set).await?;
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
