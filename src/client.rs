use crate::{consts::BASE_DOMAIN, ech::ready_tls_config, types::LogLevel};
use anyhow::Result;
use reqwest::blocking::Client;
use std::{
    sync::{LazyLock, Mutex},
    time::{Duration, Instant},
};

static CLIENT: LazyLock<Mutex<(Option<Client>, Instant)>> =
    LazyLock::new(|| Mutex::new((None, Instant::now())));

fn build_client() -> anyhow::Result<Client> {
    let tls = ready_tls_config(BASE_DOMAIN)?;
    let builder = Client::builder()
        .tls_backend_preconfigured(tls)
        .tcp_nodelay(true)
        .tcp_keepalive(Duration::from_secs(15))
        .connect_timeout(Duration::from_secs(30));
    Ok(builder.build()?)
}

pub fn client() -> Result<Client> {
    let mut state = CLIENT.lock().unwrap_or_else(|e| e.into_inner());

    let needs_build: bool = state.0.is_none() || state.1.elapsed() >= Duration::from_secs(250);

    if needs_build {
        match build_client() {
            Ok(new_client) => {
                state.0 = Some(new_client);
                state.1 = Instant::now();
            }
            Err(e) => {
                debug_log!(LogLevel::Error, "Client rebuild failed, retrying later...");
                return Err(e);
            }
        }
    }

    #[allow(clippy::expect_used)]
    Ok(state
        .0
        .as_ref()
        .expect("unpopulated client, bad code")
        .clone())
}
