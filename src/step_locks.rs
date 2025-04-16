use std::path::PathBuf;

use crate::{Result, hook::HookContext, step::RunType, step_job::StepJob};

use eyre::OptionExt;
use tokio::sync::{OwnedRwLockReadGuard, OwnedRwLockWriteGuard, OwnedSemaphorePermit};

use crate::step_context::StepContext;

#[allow(unused)]
#[derive(Debug)]
pub struct StepLocks {
    read_flocks: Vec<OwnedRwLockReadGuard<()>>,
    write_flocks: Vec<OwnedRwLockWriteGuard<()>>,
    permit: OwnedSemaphorePermit,
}

impl StepLocks {
    pub async fn lock(hook_ctx: &HookContext, ctx: &StepContext, job: &StepJob) -> Result<Self> {
        let step = &job.step;
        let file_locks = hook_ctx.file_locks.lock().await.clone();
        for dep in &step.depends {
            if !ctx.depends.is_done(dep) {
                debug!("{step}: waiting for {dep} to finish");
                ctx.depends.wait_for(dep).await?;
            }
        }
        let mut read_flocks = vec![];
        let mut write_flocks = vec![];
        let files = if step.stomp { &job.files } else { &vec![] };
        for path in files {
            let path = if let Some(dir) = &step.dir {
                PathBuf::from(dir).join(path)
            } else {
                path.to_path_buf()
            };
            match (step.stomp, job.run_type) {
                (true, _) | (_, RunType::Check(_)) => {
                    let lock = file_locks
                        .get(&path)
                        .ok_or_eyre(eyre::eyre!("file lock not found for {}", path.display()))?
                        .clone();
                    if let Ok(lock) = lock.clone().try_read_owned() {
                        read_flocks.push(lock);
                    } else {
                        debug!("{step}: waiting for {} to finish for read", path.display());
                        read_flocks.push(lock.read_owned().await);
                    }
                }
                (_, RunType::Fix) => {
                    let lock = file_locks
                        .get(&path)
                        .ok_or_eyre(eyre::eyre!("file lock not found for {}", path.display()))?
                        .clone();
                    if let Ok(lock) = lock.clone().try_write_owned() {
                        write_flocks.push(lock);
                    } else {
                        debug!("{step}: waiting for {} to finish for write", path.display());
                        write_flocks.push(lock.write_owned().await);
                    }
                }
            }
        }
        Ok(Self {
            read_flocks,
            write_flocks,
            permit: hook_ctx.semaphore.clone().acquire_owned().await?,
        })
    }
}
