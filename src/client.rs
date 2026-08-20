use crate::{consts::BASE_DOMAIN, ech::ready_tls_config, types::LogLevel};
use reqwest::blocking::Client;
use std::{
    sync::{LazyLock, Mutex},
    time::{Duration, Instant},
};

static CLIENT: LazyLock<Mutex<(Client, Instant)>> =
    LazyLock::new(|| Mutex::new((build_client_or_default(), Instant::now())));

fn build_client() -> anyhow::Result<Client> {
    let tls = ready_tls_config(BASE_DOMAIN)?;
    let builder = Client::builder()
        .tls_backend_preconfigured(tls)
        .tcp_nodelay(true)
        .tcp_keepalive(Duration::from_secs(15))
        .timeout(Duration::from_secs(30));
    Ok(builder.build()?)
}

fn build_client_or_default() -> Client {
    for attempt in 0..10 {
        match build_client() {
            Ok(c) => return c,
            Err(e) => {
                debug_log!(
                    LogLevel::Error,
                    "Initial TLS client build failed (attempt {attempt}): {e}"
                );
                std::thread::sleep(Duration::from_millis(500 * (attempt + 1) as u64));
            }
        }
    }
    panic!("failed to build HTTP client after retries");
}

pub fn client() -> Client {
    let mut state = CLIENT.lock().unwrap_or_else(|e| e.into_inner());

    if state.1.elapsed() >= Duration::from_secs(300) {
        match build_client() {
            Ok(new_client) => {
                *state = (new_client, Instant::now());
            }
            Err(e) => {
                debug_log!(
                    LogLevel::Error,
                    "Client rebuild failed, continuing with existing client: {e}"
                );
                state.1 = Instant::now() - Duration::from_secs(300) + Duration::from_secs(30);
            }
        }
    }

    state.0.clone()
}
