use std::{
    sync::{LazyLock, Mutex},
    time::{Duration, Instant},
};

use reqwest::blocking::Client;

use crate::{consts::BASE_DOMAIN, ech::ready_tls_config};

static CLIENT: LazyLock<Mutex<(Client, Instant)>> =
    LazyLock::new(|| Mutex::new((build_client(), Instant::now())));

fn build_client() -> Client {
    let tls = ready_tls_config(BASE_DOMAIN).expect("failed to ready TLS config");

    let builder = Client::builder()
        .tls_backend_preconfigured(tls)
        .tcp_nodelay(true)
        .tcp_keepalive(Duration::from_secs(15));

    builder.build().expect("failed to build HTTP client")
}

pub fn client() -> Client {
    let mut state = CLIENT.lock().expect("HTTP client lock poisoned");

    if state.1.elapsed() >= Duration::from_secs(300) {
        *state = (build_client(), Instant::now());
    }

    state.0.clone()
}
