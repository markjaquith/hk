use crate::{
    step::{RunType, Step},
    tera,
};
use indexmap::IndexMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, OwnedRwLockWriteGuard, RwLock, Semaphore};

pub struct StepContext {
    pub step: Step,
    pub run_type: RunType,
    pub files: Vec<PathBuf>,
    pub file_locks: IndexMap<PathBuf, Arc<RwLock<()>>>,
    pub semaphore: Arc<Semaphore>,
    pub failed: Arc<Mutex<bool>>,
    pub tctx: tera::Context,
    pub has_files_in_contention: bool,
    #[allow(unused)]
    pub depend_self: Option<OwnedRwLockWriteGuard<()>>,
}

impl Clone for StepContext {
    fn clone(&self) -> Self {
        Self {
            step: self.step.clone(),
            run_type: self.run_type,
            files: self.files.clone(),
            file_locks: self.file_locks.clone(),
            semaphore: self.semaphore.clone(),
            failed: self.failed.clone(),
            tctx: self.tctx.clone(),
            has_files_in_contention: self.has_files_in_contention,
            depend_self: None,
        }
    }
}
