use std::{fs, path::Path};
use uuid::Uuid;

use crate::types::LogLevel;

/// Decides the identity for the current session and creates an identity file, and stores the initialized
/// identity for future sessions.
pub fn init_ident_and_file(state_dir: Option<&Path>) -> String {
    let fallback = || create_new_ident(false);
    let Some(state_dir) = state_dir else {
        debug_log!(
            LogLevel::Warn,
            "State directory indeterminate; using dyn ident..."
        );
        return fallback();
    };

    let ident_fp = state_dir.join(".ident");
    if !ident_fp.try_exists().unwrap_or(false) {
        let i = create_new_ident(true);
        if let Err(e) = fs::write(&ident_fp, &i) {
            debug_log!(
                LogLevel::Error,
                ".ident write failure: {e}; using dyn ident..."
            );
            fallback()
        } else {
            debug_log!(LogLevel::Ok, "Generated new static identity: {i}");
            i
        }
    } else {
        if let Ok(d) = fs::read_to_string(&ident_fp) {
            debug_log!(LogLevel::Ok, "Using ident from previous session: {d}");
            d
        } else {
            debug_log!(
                LogLevel::Error,
                "Failed to read previously generated ident, using dyn ident."
            );
            fallback()
        }
    }
}

/// Creates a new identity for the current-running instance.
fn create_new_ident(st: bool) -> String {
    let mut ident = Uuid::new_v4().to_string();
    ident.push(';');
    ident.push_str(std::env::consts::ARCH);
    ident.push(';');
    ident.push_str(if st { "st" } else { "dyn" });
    ident
}
