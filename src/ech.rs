use std::sync::Arc;
use std::{sync::LazyLock, time::Duration};

use anyhow::Result;
use base64::{Engine, engine::general_purpose::STANDARD};
use reqwest::blocking::Client;
use rustls::client::{ClientConfig, EchConfig, EchMode};
use rustls::crypto::aws_lc_rs;
use rustls_platform_verifier::BuilderVerifierExt;

static DOH_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .timeout(Duration::from_millis(1500))
        .build()
        .expect("failed to build DoH client")
});

pub fn ready_tls_config(domain: &str) -> Result<ClientConfig> {
    let dns: serde_json::Value = DOH_CLIENT
        .get(format!(
            "https://1.1.1.1/dns-query?name={domain}&type=HTTPS"
        ))
        .header("Accept", "application/dns-json")
        .send()?
        .json()?;

    let data = dns["Answer"][0]["data"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing HTTPS record"))?;

    let ech_b64 = data
        .split_whitespace()
        .find_map(|x| x.strip_prefix("ech="))
        .ok_or_else(|| anyhow::anyhow!("missing ECH config"))?;

    let ech_bytes = STANDARD.decode(ech_b64)?;

    let ech = EchConfig::new(
        ech_bytes.as_slice().into(),
        &[rustls::crypto::aws_lc_rs::hpke::DH_KEM_X25519_HKDF_SHA256_AES_128],
    )?;

    let provider = Arc::new(aws_lc_rs::default_provider());
    let tls = ClientConfig::builder_with_provider(provider)
        .with_ech(EchMode::Enable(ech))?
        .with_platform_verifier()?
        .with_no_client_auth();

    Ok(tls)
}
