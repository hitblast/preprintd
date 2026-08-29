use crate::{
    consts::BASE_DOMAIN,
    doh::{DohResolver, ready_tls_config},
    types::LogLevel,
};
use anyhow::Result;
use reqwest::blocking::Client;
use std::{
    sync::{Arc, LazyLock, Mutex},
    time::{Duration, Instant},
};

static CLIENT: LazyLock<Mutex<(Option<Client>, Instant)>> =
    LazyLock::new(|| Mutex::new((None, Instant::now())));

fn build_client() -> anyhow::Result<Client> {
    let resolver = Arc::new(DohResolver);
    let mut builder: reqwest::blocking::ClientBuilder = Client::builder()
        .dns_resolver(resolver)
        .tcp_nodelay(true)
        .tcp_keepalive(Duration::from_secs(15))
        .connect_timeout(Duration::from_secs(30));

    if let Ok(cfg) = ready_tls_config(BASE_DOMAIN) {
        debug_log!(
            LogLevel::Ok,
            "ECH resolved! Proceeding with custom TLS backend..."
        );
        builder = builder.tls_backend_preconfigured(cfg);
    } else {
        debug_log!(
            LogLevel::Warn,
            "Failed to resolve ECH! Proceeding with bare configuration..."
        );
    }

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
