use std::fs;
use std::path::Path;
use uuid::Uuid;

use crate::types::LogLevel;

/// Decides the identity for the given
pub fn decide_ident(ident_fp: Option<PathBuf>) -> String {
    let Some(ident_fp) = ident_fp else {
        debug_log!(
            LogLevel::Warn,
            "State directory indeterminate; using dyn ident..."
        );
        return create_new_ident(false);
    };

    if !ident_fp.try_exists().unwrap_or(false) {
        let i = create_new_ident(true);
        if let Err(e) = fs::write(&ident_fp, &i) {
            debug_log!(
                LogLevel::Error,
                ".ident write failure: {e}; using dyn ident..."
            );
            create_new_ident(false)
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
            create_new_ident(false)
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
