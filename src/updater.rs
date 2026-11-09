use std::env;
use std::sync::{LazyLock, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Result, anyhow};
use self_update::backends::github::Update;
use self_update::cargo_crate_version;

use crate::types::LogLevel;

pub static LAST_UPDATE_CHECK: LazyLock<Mutex<Option<u64>>> = LazyLock::new(|| Mutex::new(None));

fn get_update_backend() -> Result<Update> {
    let update = self_update::backends::github::Update::configure()
        .repo_owner("hitblast")
        .repo_name("preprintd")
        .bin_name("preprintd")
        .target(self_update::get_target())
        .current_version(cargo_crate_version!())
        .no_confirm(true)
        .show_output(false)
        .build()?;

    Ok(update)
}

fn is_latest() -> Result<bool> {
    let mut last_check = LAST_UPDATE_CHECK
        .lock()
        .map_err(|_| anyhow!("last update check mutex lock poisoned"))?;

    let now = SystemTime::now();
    let duration_since_last_check = now.duration_since(UNIX_EPOCH)?.as_secs();

    let is_latest = get_update_backend()?.is_update_available()?.is_none();
    *last_check = Some(duration_since_last_check);

    Ok(is_latest)
}

fn apply_update_and_restart() -> Result<bool> {
    let status = get_update_backend()?.update()?;

    if status.is_updated() {
        debug_log!(
            LogLevel::Warn,
            "Updated to version {}. Restarting daemon...",
            status.version()
        );

        self_update::restart::restart()?;
    }

    Ok(status.is_updated())
}

/// Spawns a background thread that polls for updates every 24 hours.
pub fn spawn_update_thread() {
    thread::spawn(|| {
        debug_log!(LogLevel::Ok, "Updater thread started.");

        let check_interval = Duration::from_hours(24);

        loop {
            match is_latest() {
                Ok(true) => {}
                Ok(false) => {
                    debug_log!(LogLevel::Warn, "New version detected. Applying update...");
                    if let Err(err) = apply_update_and_restart() {
                        debug_log!(LogLevel::Warn, "Failed to apply update: {}", err);
                    }
                }
                Err(err) => {
                    debug_log!(LogLevel::Warn, "Failed checking for updates: {}", err);
                }
            }

            thread::sleep(check_interval);
        }
    });
}
