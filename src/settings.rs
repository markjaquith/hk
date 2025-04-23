use std::{
    num::NonZero,
    sync::{LazyLock, Mutex},
};

use indexmap::IndexSet;

use crate::env;

#[derive(Debug)]
pub struct Settings {
    pub jobs: NonZero<usize>,
    pub enabled_profiles: IndexSet<String>,
    pub disabled_profiles: IndexSet<String>,
    pub fail_fast: bool,
}

static JOBS: LazyLock<Mutex<Option<NonZero<usize>>>> = LazyLock::new(Default::default);
static ENABLED_PROFILES: LazyLock<Mutex<Option<IndexSet<String>>>> =
    LazyLock::new(Default::default);
static DISABLED_PROFILES: LazyLock<Mutex<Option<IndexSet<String>>>> =
    LazyLock::new(Default::default);

impl Settings {
    pub fn get() -> &'static Self {
        static SETTINGS: LazyLock<Settings> = LazyLock::new(Default::default);
        &SETTINGS
    }

    pub fn with_profiles(profiles: &[String]) {
        for profile in profiles {
            if profile.starts_with('!') {
                let profile = profile.strip_prefix('!').unwrap();
                let mut disabled_profiles = DISABLED_PROFILES.lock().unwrap();
                disabled_profiles
                    .get_or_insert_default()
                    .insert(profile.to_string());
            } else {
                let mut enabled_profiles = ENABLED_PROFILES.lock().unwrap();
                enabled_profiles
                    .get_or_insert_default()
                    .insert(profile.to_string());
                let mut disabled_profiles = DISABLED_PROFILES.lock().unwrap();
                disabled_profiles
                    .get_or_insert_default()
                    .shift_remove(profile);
            }
        }
    }

    pub fn set_jobs(jobs: NonZero<usize>) {
        *JOBS.lock().unwrap() = Some(jobs);
    }
}

impl Default for Settings {
    fn default() -> Self {
        let disabled_profiles: IndexSet<String> = DISABLED_PROFILES
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| {
                env::HK_PROFILE
                    .iter()
                    .filter(|p| p.starts_with('!'))
                    .map(|p| p.strip_prefix('!').unwrap().to_string())
                    .collect()
            });
        let enabled_profiles: IndexSet<String> =
            ENABLED_PROFILES.lock().unwrap().clone().unwrap_or_else(|| {
                env::HK_PROFILE
                    .iter()
                    .filter(|p| !disabled_profiles.contains(*p))
                    .filter(|p| !p.starts_with('!'))
                    .map(|p| p.to_string())
                    .collect()
            });
        Self {
            jobs: JOBS.lock().unwrap().unwrap_or(*env::HK_JOBS),
            enabled_profiles,
            disabled_profiles,
            fail_fast: *env::HK_FAIL_FAST,
        }
    }
}
