use std::env;
use std::process::Command;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use self_update::cargo_crate_version;

use crate::types::LogLevel;

fn is_latest() -> Result<bool> {
    let releases = self_update::backends::github::ReleaseList::configure()
        .repo_owner("hitblast")
        .repo_name("preprintd")
        .build()?
        .fetch()?;

    let releases_str = releases
        .clone()
        .into_iter()
        .map(|f| f.version().to_string())
        .collect::<Vec<String>>();

    debug_log!(LogLevel::Ok, "Found releases:");
    debug_log!(LogLevel::Ok, "{:?}\n", releases_str);

    let latest_version = releases
        .latest()
        .map(|release| release.version())
        .unwrap_or_else(|| cargo_crate_version!());

    Ok(latest_version == cargo_crate_version!())
}

fn apply_update_and_restart() -> Result<bool> {
    let status = self_update::backends::github::Update::configure()
        .repo_owner("hitblast")
        .repo_name("preprintd")
        .bin_name("preprintd")
        .show_download_progress(false)
        .current_version(cargo_crate_version!())
        .build()?
        .update()?;

    if status.is_updated() {
        debug_log!(
            LogLevel::Warn,
            "Updated to version {}. Restarting daemon...",
            status.version()
        );

        // restart logic
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            let args: Vec<String> = env::args().collect();
            let current_exe = env::current_exe()?;

            // re-exec replaces the current process image in-place
            // (preserves PID under systemd)
            //
            let err = Command::new(&current_exe).args(&args[1..]).exec();
            debug_log!(LogLevel::Error, "Failed to re-exec updated daemon: {err}");
            std::process::exit(1);
        }

        #[cfg(not(unix))]
        {
            // exit cleanly for process managers (systemd/docker) configured with Restart=always
            std::process::exit(0);
        }
    }

    Ok(false)
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
