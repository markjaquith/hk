use crate::{Result, step::RunType};
use std::sync::Arc;

use indexmap::IndexMap;
use tokio::sync::{OwnedRwLockReadGuard, OwnedRwLockWriteGuard, OwnedSemaphorePermit, RwLock};

use crate::step_context::StepContext;

#[allow(unused)]
pub struct StepLocks {
    read_flocks: Vec<OwnedRwLockReadGuard<()>>,
    write_flocks: Vec<OwnedRwLockWriteGuard<()>>,
    permit: OwnedSemaphorePermit,
}

impl StepLocks {
    pub fn try_lock(
        ctx: &StepContext,
        depends: &IndexMap<String, Arc<RwLock<()>>>,
        permit: OwnedSemaphorePermit,
    ) -> Option<Self> {
        let step = &ctx.step;
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
            match (ctx.step.stomp, ctx.run_type) {
                (_, RunType::Run) => {}
                (true, _) | (_, RunType::Check(_)) => match lock.clone().try_read_owned() {
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

    pub async fn lock(
        ctx: &StepContext,
        permit: OwnedSemaphorePermit,
        depends: &IndexMap<String, Arc<RwLock<()>>>,
    ) -> Result<Self> {
        if let Some(locks) = Self::try_lock(ctx, depends, permit) {
            return Ok(locks);
        }
        let mut read_flocks = vec![];
        let mut write_flocks = vec![];
        for (_name, depends) in depends.iter() {
            read_flocks.push(depends.clone().read_owned().await);
        }
        for (_path, lock) in ctx.file_locks.iter() {
            let lock = lock.clone();
            match (ctx.step.stomp, ctx.run_type) {
                (_, RunType::Run) => {}
                (true, _) | (_, RunType::Check(_)) => {
                    read_flocks.push(lock.clone().read_owned().await)
                }
                (_, RunType::Fix) => write_flocks.push(lock.clone().write_owned().await),
            }
        }
        Ok(Self {
            read_flocks,
            write_flocks,
            permit: ctx.semaphore.clone().acquire_owned().await?,
        })
    }
}
