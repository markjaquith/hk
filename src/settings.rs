use std::{
    num::NonZero,
    sync::{LazyLock, Mutex},
};

use indexmap::IndexSet;

use crate::env;

#[derive(Debug)]
pub struct Settings {
    jobs: Mutex<NonZero<usize>>,
    enabled_profiles: Mutex<IndexSet<String>>,
    disabled_profiles: Mutex<IndexSet<String>>,
}

impl Settings {
    pub fn get() -> &'static Self {
        static SETTINGS: LazyLock<Settings> = LazyLock::new(Default::default);
        &SETTINGS
    }

    pub fn with_profiles(&self, profiles: &[String]) -> &Self {
        for profile in profiles {
            if profile.starts_with('!') {
                let profile = profile.strip_prefix('!').unwrap();
                self.disabled_profiles
                    .lock()
                    .unwrap()
                    .insert(profile.to_string());
                self.enabled_profiles.lock().unwrap().shift_remove(profile);
            } else {
                self.enabled_profiles
                    .lock()
                    .unwrap()
                    .insert(profile.to_string());
                self.disabled_profiles.lock().unwrap().shift_remove(profile);
            }
        }
        self
    }

    pub fn enabled_profiles(&self) -> IndexSet<String> {
        self.enabled_profiles.lock().unwrap().clone()
    }

    pub fn set_jobs(&self, jobs: NonZero<usize>) {
        *self.jobs.lock().unwrap() = jobs;
    }

    pub fn jobs(&self) -> NonZero<usize> {
        *self.jobs.lock().unwrap()
    }
}

impl Default for Settings {
    fn default() -> Self {
        let disabled_profiles: IndexSet<String> = env::HK_PROFILE
            .iter()
            .filter(|p| p.starts_with('!'))
            .map(|p| p.strip_prefix('!').unwrap().to_string())
            .collect();
        let enabled_profiles: IndexSet<String> = env::HK_PROFILE
            .iter()
            .filter(|p| !disabled_profiles.contains(*p))
            .filter(|p| !p.starts_with('!'))
            .map(|p| p.to_string())
            .collect();
        Self {
            jobs: Mutex::new(*env::HK_JOBS),
            enabled_profiles: Mutex::new(enabled_profiles),
            disabled_profiles: Mutex::new(disabled_profiles),
        }
    }
}
