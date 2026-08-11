use std::fs;
use std::path::Path;

use uuid::Uuid;

use crate::DEBUG;
use crate::types::LogLevel;

pub fn decide_ident(p: &Path) -> String {
    let p = p.join(".ident");

    if !p.try_exists().unwrap_or(false) {
        let i = create_new_ident(true);
        if !p
            .parent()
            .map(|f| f.try_exists().unwrap_or(false))
            .unwrap_or(false)
        {
            debug_log!(
                LogLevel::Ok,
                "State directory does not exist, attempting to create one..."
            );
            if let Err(_) = fs::create_dir_all(p.parent().expect("invalid ident path")) {
                debug_log!(
                    LogLevel::Error,
                    "Failed to create state directory, using dyn ident."
                );
                return create_new_ident(false);
            }
        }

        if let Err(e) = fs::write(&p, &i) {
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
        if let Ok(d) = fs::read_to_string(&p) {
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

pub fn create_new_ident(st: bool) -> String {
    let mut ident = Uuid::new_v4().to_string();
    ident.push(';');
    ident.push_str(std::env::consts::ARCH);
    ident.push(';');
    ident.push_str(if st { "static" } else { "dynamic" });
    ident
}
