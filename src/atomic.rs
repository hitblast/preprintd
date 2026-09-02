use std::sync::atomic::{AtomicBool, AtomicUsize};

pub static JOBS_COMPLETED: AtomicUsize = AtomicUsize::new(0);
pub static IS_ECH_DISABLED: AtomicBool = AtomicBool::new(false);
