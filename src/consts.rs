//! Sensible constants for preprintd, you ask for? They all live here.
//!
pub const DEF_LPR_PORT: u16 = 515;
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NUL: [u8; 1] = [0u8];

pub const DEF_API_URL: &str = "https://api.preconnect.app/";
