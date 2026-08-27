//! Note that this file might not contain *all* the types in the entire codebase.
//! This module is focused more on the custom types used in main.rs itself. Other modules may
//! self-contain their own types.
//!
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: Option<String>,
    pub printer_host: Option<String>,
    pub printer_queue: Option<String>,
    pub timeout: Option<f64>,

    pub q_cmd: Option<String>,
    pub cf_hdr: Option<String>,
    pub ctl: Option<String>,
    pub df_hdr: Option<String>,
    pub payload: Option<String>,
}

pub enum LogLevel {
    Ok,
    Warn,
    Error,
}

#[derive(Serialize)]
pub struct ClaimJobRequestBody<'a> {
    pub id: &'a str,
}
#[derive(Deserialize)]
pub struct ClaimJobResponseBody {
    pub claimed: Option<bool>,
}
