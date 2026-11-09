use crate::{AGENT, client::client, consts::VERSION, types::LogLevel};
use anyhow::Result;
use serde::Deserialize;
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

pub static LAST_UPDATE_CHECK: AtomicU64 = AtomicU64::new(0);

const CRATE_URL: &str = "https://crates.io/api/v1/crates/preprintd";
const CHECK_EVERY_MS: u64 = 6 * 60 * 60 * 1000;

#[derive(Deserialize)]
struct CrateResponse {
    #[serde(rename = "crate")]
    krate: CrateInfo,
}

#[derive(Deserialize)]
struct CrateInfo {
    max_stable_version: String,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

fn version_parts(version: &str) -> Vec<u64> {
    version
        .split('.')
        .map(|part| part.parse().unwrap_or(0))
        .collect()
}

fn latest_version() -> Result<String> {
    let response: CrateResponse = client()?
        .get(CRATE_URL)
        .header("User-Agent", AGENT.as_str())
        .send()?
        .error_for_status()?
        .json()?;
    Ok(response.krate.max_stable_version)
}

pub fn check_for_update_if_due() {
    let last = LAST_UPDATE_CHECK.load(Ordering::Relaxed);
    if last != 0 && now_ms().saturating_sub(last) < CHECK_EVERY_MS {
        return;
    }
    match latest_version() {
        Ok(latest) => {
            LAST_UPDATE_CHECK.store(now_ms(), Ordering::Relaxed);
            if version_parts(&latest) > version_parts(VERSION) {
                debug_log!(
                    LogLevel::Warn,
                    "Update available: {VERSION} -> {latest} (cargo install preprintd)"
                );
            } else {
                debug_log!(LogLevel::Ok, "Up to date ({VERSION}).");
            }
        }
        Err(e) => {
            debug_log!(LogLevel::Warn, "Update check failed: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::version_parts;

    #[test]
    fn compares_versions_numerically() {
        assert!(version_parts("0.10.0") > version_parts("0.9.4"));
        assert!(version_parts("0.9.4") > version_parts("0.8.0"));
        assert!(version_parts("0.8.0") <= version_parts("0.8.0"));
    }
}
