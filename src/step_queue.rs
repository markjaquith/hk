use crate::step::Step;
use crate::step_job::StepJob;

use crate::Result;
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{glob, step::RunType};

/// Takes a list of steps and files as input and builds a queue of jobs that would need to be
/// executed by StepScheduler
///
/// This is kept outside of the Scheduler so the logic here is pure where the scheduler deals with
/// parallel execution synchronization.
pub struct StepQueue {
    pub groups: Vec<Vec<StepJob>>,
}

pub struct StepQueueBuilder {
    steps: Vec<Arc<Step>>,
    files: Vec<PathBuf>,
    run_type: RunType,
}

impl StepQueueBuilder {
    pub fn new(steps: Vec<Arc<Step>>, files: Vec<PathBuf>, run_type: RunType) -> Self {
        Self {
            steps,
            files,
            run_type,
        }
    }

    pub fn build(&self) -> Result<StepQueue> {
        // groups is a list of list of steps which are separated by exclusive steps
        // any exclusive step will be in a group by itself
        let groups = self
            .steps
            .iter()
            .fold(vec![], |mut groups, step| {
                if step.exclusive || groups.is_empty() {
                    groups.push(vec![]);
                }
                groups.last_mut().unwrap().push(step);
                if step.exclusive {
                    groups.push(vec![]);
                }
                groups
            })
            .into_iter()
            .filter(|group| !group.is_empty())
            .map(|group| self.build_queue_for_group(&group))
            .collect::<Result<_>>()?;

        Ok(StepQueue { groups })
    }

    fn build_queue_for_group(&self, steps: &[&Arc<Step>]) -> Result<Vec<StepJob>> {
        let mut queue = vec![];
        for step in steps {
            let Some(run_type) = step.available_run_type(self.run_type) else {
                debug!("{step}: skipping step due to no available run type");
                continue;
            };
            // Check if step should be skipped based on HK_SKIP_STEPS
            if crate::env::HK_SKIP_STEPS.contains(&step.name) {
                debug!("{step}: skipping step due to HK_SKIP_STEPS");
                continue;
            }
            if !step.is_profile_enabled() {
                debug!("{step}: skipping step due to profile not being enabled");
                continue;
            }
            queue.push(
                step.build_step_jobs(&self.files, run_type)?
                    .unwrap_or_default(),
            );
        }
        let mut q = vec![];
        // round robin through the steps to avoid just 1 step running
        while !queue.is_empty() {
            for jobs in queue.iter_mut() {
                if let Some(job) = jobs.pop() {
                    q.push(job);
                }
            }
            queue.retain(|jobs| !jobs.is_empty());
        }

        if q.iter().any(|j| j.check_first) {
            let files_in_contention = self.files_in_contention(steps, &self.files)?;
            for job in q.iter_mut().filter(|j| j.check_first) {
                // only set check_first if there are any files in contention
                job.check_first = job.files.iter().any(|f| files_in_contention.contains(f));
            }
        }
        Ok(q)
    }

    fn files_in_contention(
        &self,
        steps: &[&Arc<Step>],
        files: &[PathBuf],
    ) -> Result<HashSet<PathBuf>> {
        let step_map: HashMap<&str, &Step> = steps
            .iter()
            .map(|step| (step.name.as_str(), &***step))
            .collect();
        let files_by_step: HashMap<&str, Vec<PathBuf>> = steps
            .iter()
            .map(|step| {
                let files = glob::get_matches(step.glob.as_ref().unwrap_or(&vec![]), files)?;
                Ok((step.name.as_str(), files))
            })
            .collect::<Result<_>>()?;
        let mut steps_per_file: HashMap<&Path, Vec<&Step>> = Default::default();
        for (step_name, files) in files_by_step.iter() {
            for file in files {
                let step = step_map.get(step_name).unwrap();
                steps_per_file.entry(file.as_path()).or_default().push(step);
            }
        }

        let mut files_in_contention = HashSet::new();
        for (file, steps) in steps_per_file.iter() {
            if steps
                .iter()
                .any(|step| step.available_run_type(self.run_type) == Some(RunType::Fix))
            {
                files_in_contention.insert(file.to_path_buf());
            }
        }

        Ok(files_in_contention)
    }
}
