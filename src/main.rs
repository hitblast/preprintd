use std::fs;
/*
 * preprintd - Printer swarm listener/worker implementation for PreConnect.
 * Copyright (C) 2026  Anindya Shiddhartha & contributors
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 *
 */
use std::path::PathBuf;
use std::sync::LazyLock;
use std::{
    env,
    io::{BufRead, BufReader},
    net::{TcpStream, ToSocketAddrs},
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    thread::sleep,
    time::{Duration, Instant},
};

#[macro_use]
mod macros;

mod client;
mod consts;
mod crypto;
mod doh;
mod ident;
mod tcp_extras;
mod types;
mod zbus;

use anyhow::Result;
use reqwest::{
    StatusCode,
    header::{HeaderMap, HeaderValue},
};
use socket2::SockRef;
use tcp_extras::TcpExtras;

use crate::ident::decide_ident;

use crate::types::{ClaimJobRequestBody, ClaimJobResponseBody};
#[cfg(target_os = "linux")]
use crate::zbus::acquire_sleep_inhibitor;

use crate::{
    client::client,
    consts::BASE_URL,
    crypto::decrypt,
    types::{Job, LogLevel},
};

static DEBUG: LazyLock<bool> = LazyLock::new(|| env::args().any(|arg| arg == "--debug"));
#[cfg(target_os = "linux")]
static INHIBIT: LazyLock<bool> = LazyLock::new(|| env::args().any(|arg| arg == "--inhibit"));

static STATE_DIR: LazyLock<Option<PathBuf>> = LazyLock::new(|| {
    let Ok(p) = env::var("STATE_DIRECTORY") else {
        return None;
    };

    let p = PathBuf::from(p);

    if !p.try_exists().unwrap_or(false)
        && let Err(e) = fs::create_dir_all(&p)
    {
        debug_log!(
            LogLevel::Error,
            "Failed to create state directory (supplied via STATE_DIRECTORY): {e}"
        );
        return None;
    }

    Some(p)
});

pub static WORKER_IDENT: LazyLock<String> = LazyLock::new(|| decide_ident(STATE_DIR.as_deref()));

static ALIAS: LazyLock<String> =
    LazyLock::new(|| env::var("ALIAS").unwrap_or("preprintd".to_string()));
#[allow(clippy::expect_used)]
static WORKER_KEY: LazyLock<String> =
    LazyLock::new(|| env::var("WORKER_KEY").expect("missing WORKER_KEY env var"));
static AGENT: LazyLock<String> = LazyLock::new(|| format!("{}/{}", ALIAS.as_str(), VERSION));
#[allow(clippy::expect_used)]
static DEF_HOST: LazyLock<String> =
    LazyLock::new(|| env::var("DEF_HOST").expect("missing DEF_HOST env var"));
#[allow(clippy::expect_used)]
static DEF_QUEUE: LazyLock<String> =
    LazyLock::new(|| env::var("DEF_QUEUE").expect("missing DEF_QUEUE env var"));

static PRINT_LOCK: Mutex<()> = Mutex::new(());
static JOBS_COMPLETED: AtomicUsize = AtomicUsize::new(0);

const VERSION: &str = env!("CARGO_PKG_VERSION");
const DEF_LPR_PORT: u16 = 515;
const NUL: [u8; 1] = [0u8];

fn is_online(host: &str) -> Result<bool> {
    if host.is_empty() {
        return Ok(false);
    }
    sock!(s, host, DEF_LPR_PORT, Duration::from_millis(800));

    if let Ok(conn) = s {
        let _ = conn.shutdown(std::net::Shutdown::Both);
    } else {
        return Ok(false);
    }

    Ok(true)
}

fn hdrs() -> Result<HeaderMap> {
    let mut map: HeaderMap = HeaderMap::new();
    let jobs: String = JOBS_COMPLETED.load(Ordering::Relaxed).to_string();

    map.insert("User-Agent", HeaderValue::from_str(&AGENT)?);
    map.insert("X-Worker-Key", HeaderValue::from_str(&WORKER_KEY)?);
    map.insert("X-Worker-Jobs", HeaderValue::from_str(&jobs)?);
    map.insert("X-Worker-Ident", HeaderValue::from_str(&WORKER_IDENT)?);

    Ok(map)
}

fn claim_job(id: &str) -> Result<bool> {
    let body = ClaimJobRequestBody { id };

    let resp = client()?
        .post(format!("{BASE_URL}/print/claim"))
        .body(serde_json::to_string(&body)?)
        .header("Content-Type", "application/json")
        .headers(hdrs()?)
        .timeout(Duration::from_secs(10))
        .send();

    let claim: bool = match resp {
        Ok(r) => {
            if r.status() != StatusCode::OK {
                debug_log!(
                    LogLevel::Warn,
                    "Status code not OK, so skipping on this job..."
                );
                return Ok(false);
            }

            let Ok(value) = r.json::<ClaimJobResponseBody>() else {
                debug_log!(LogLevel::Warn, "Parsing failed, so skipping on this job...");
                return Ok(false);
            };
            value.claimed.unwrap_or(false)
        }
        Err(e) => {
            debug_log!(LogLevel::Error, "(Send) /print/claim: {e}");
            false
        }
    };

    if claim {
        debug_log!(LogLevel::Ok, "Claimed new job!");
    } else {
        debug_log!(LogLevel::Warn, "Skipping on this job...");
    }

    Ok(claim)
}

fn handle(job: Job) -> Result<()> {
    let job_id = {
        if let Some(j_id) = job.id.as_deref()
            && !j_id.is_empty()
        {
            j_id
        } else {
            debug_log!(
                LogLevel::Error,
                "Empty job ID received from job description; skipping job..."
            );
            return Ok(());
        }
    };

    let host = job
        .printer_host
        .as_deref()
        .filter(|host| !host.is_empty())
        .unwrap_or(&DEF_HOST);

    let queue_name = job
        .printer_queue
        .as_deref()
        .filter(|queue| !queue.is_empty())
        .unwrap_or(&DEF_QUEUE);

    if !is_online(host)? || !(claim_job(job_id)?) {
        return Ok(());
    }

    let q_cmd = decrypt(job.q_cmd.as_deref(), job_id)?;
    let cf_hdr = decrypt(job.cf_hdr.as_deref(), job_id)?;
    let ctl = decrypt(job.ctl.as_deref(), job_id)?;
    let df_hdr = decrypt(job.df_hdr.as_deref(), job_id)?;
    let payload = decrypt(job.payload.as_deref(), job_id)?;

    debug_log!(
        LogLevel::Ok,
        "Handling job for {host}:{queue_name} (payload size: {} bytes)",
        payload.len()
    );

    let timeout = Duration::from_secs_f64(
        job.timeout
            .filter(|timeout| timeout.is_finite() && *timeout > 0.0)
            .unwrap_or(60.0),
    );

    sock!(s, host, DEF_LPR_PORT, timeout);
    let mut socket = match s {
        Ok(s) => s,
        Err(e) => {
            debug_log!(
                LogLevel::Error,
                "Failed to connect to {host}:{DEF_LPR_PORT}: {e}"
            );
            return Ok(());
        }
    };

    let transferred = (|| -> std::io::Result<bool> {
        socket.set_nodelay(true)?;
        socket.set_read_timeout(Some(timeout))?;
        socket.set_write_timeout(Some(timeout))?;

        SockRef::from(&socket).set_send_buffer_size(65_536)?;

        Ok(socket.send_buf(&q_cmd)?
            && socket.recv_ack()?
            && socket.send_buf(&cf_hdr)?
            && socket.recv_ack()?
            && socket.send_buf(&ctl)?
            && socket.send_buf(&NUL)?
            && socket.recv_ack()?
            && socket.send_buf(&df_hdr)?
            && socket.recv_ack()?
            && socket.send_buf(&payload)?
            && socket.send_buf(&NUL)?
            && socket.recv_ack()?)
    })();

    match transferred {
        Ok(true) => {
            debug_log!(LogLevel::Ok, "Job transferred successfully.");
            JOBS_COMPLETED.fetch_add(1, Ordering::Relaxed);
        }
        Ok(false) => {
            debug_log!(
                LogLevel::Warn,
                "Unknown failure while sending printer socket."
            );
        }
        Err(e) => {
            debug_log!(LogLevel::Error, "Printer transfer failed: {e}");
        }
    };

    let _ = socket.shutdown(std::net::Shutdown::Both);
    debug_log!(LogLevel::Ok, "Shutting down current socket connection.");

    Ok(())
}

fn stream() -> Result<()> {
    let resp = match client()?
        .get(format!("{BASE_URL}/printer"))
        .header("Accept", "text/event-stream")
        .headers(hdrs()?)
        .send()
    {
        Ok(r) => {
            if r.status() == StatusCode::UNAUTHORIZED {
                debug_log!(
                    LogLevel::Error,
                    "worker key invalid ({})",
                    r.status().as_u16()
                );
                return Ok(());
            }

            if r.status() != StatusCode::OK {
                debug_log!(
                    LogLevel::Error,
                    "(Status) printer stream endpoint: {}",
                    r.status()
                );
                return Ok(());
            }

            r
        }

        Err(e) => {
            debug_log!(LogLevel::Error, "(Send) printer stream endpoint: {e}");
            return Ok(());
        }
    };

    let mut reader = BufReader::new(resp);
    let mut line = String::new();

    loop {
        line.clear();

        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                if line.starts_with(':') {
                    continue;
                } else if let Some(data) = line.strip_prefix("data: ")
                    && let Ok(value) = serde_json::from_str::<Job>(data)
                {
                    debug_log!(LogLevel::Ok, "Data match for new job!");

                    if let Err(e) = std::thread::Builder::new().spawn(move || {
                        let _guard = PRINT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
                        if let Err(panic) =
                            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                let _ = handle(value);
                            }))
                        {
                            debug_log!(LogLevel::Error, "Job handler panicked: {:?}", panic);
                        }
                    }) {
                        debug_log!(LogLevel::Error, "Failed to spawn print thread: {e}");
                    }
                }
            }
            Err(e) => {
                debug_log!(LogLevel::Error, "Printer stream read error: {e}");
                let mut src = std::error::Error::source(&e);
                while let Some(s) = src {
                    debug_log!(LogLevel::Error, "  caused by: {s}");
                    src = s.source();
                }
                break;
            }
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    #[cfg(target_os = "linux")]
    let _sleep_inhibitor = if *INHIBIT {
        Some(acquire_sleep_inhibitor()?)
    } else {
        None
    };

    let mut iter_count = 0;
    let mut delay = 1.0_f64;

    loop {
        let started_at = Instant::now();
        let result = stream();

        let long_stream = started_at.elapsed() > Duration::from_secs(10);
        delay = if result.is_ok() && long_stream {
            debug_log!(
                LogLevel::Ok,
                "{iter_count} end: Refreshing printer event stream connection..."
            );
            1.0
        } else {
            let next_delay = (delay * 2.0).min(8.0);
            debug_log!(
                LogLevel::Warn,
                "{iter_count} end: Re-establishing stream connection (backoff: {next_delay:.1}s)..."
            );
            next_delay
        };

        sleep(Duration::from_secs_f64(delay));
        iter_count += 1;
    }
}
