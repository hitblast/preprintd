use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, LazyLock};

use anyhow::{Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use reqwest::blocking::Client;
use reqwest::dns::{Addrs, Resolve, Resolving};
use rustls::client::{ClientConfig, EchConfig, EchMode};
use rustls::crypto::aws_lc_rs;
use rustls_platform_verifier::BuilderVerifierExt;

#[derive(Debug, serde::Deserialize)]
struct DohResponse {
    #[serde(rename = "Answer")]
    answer: Option<Vec<DohAnswer>>,
}

#[derive(Debug, serde::Deserialize)]
struct DohAnswer {
    #[serde(rename = "type")]
    record_type: u16,
    data: String,
}

static DOH_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    #[allow(clippy::expect_used)]
    Client::builder()
        .build()
        .expect("failed to build DoH client")
});

pub struct DohResolver;

impl DohResolver {
    pub fn new() -> Self {
        Self
    }
}

impl Resolve for DohResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> Resolving {
        let host = name.as_str().to_owned();

        let result: Result<Addrs, Box<dyn std::error::Error + Send + Sync>> = (|| {
            let dns: DohResponse = DOH_CLIENT
                .get(format!("https://1.1.1.1/dns-query?name={host}&type=A"))
                .header("Accept", "application/dns-json")
                .send()?
                .json()?;

            let addrs: Vec<SocketAddr> = dns
                .answer
                .unwrap_or_default()
                .into_iter()
                .filter(|record| record.record_type == 1)
                .filter_map(|record| record.data.parse::<IpAddr>().ok().map(|ip| (ip, 0).into()))
                .collect();

            Ok(Box::new(addrs.into_iter()) as Addrs)
        })();

        Box::pin(async move { result })
    }
}

pub fn ready_tls_config(domain: &str) -> Result<ClientConfig> {
    let dns: DohResponse = DOH_CLIENT
        .get(format!(
            "https://1.1.1.1/dns-query?name={domain}&type=HTTPS"
        ))
        .header("Accept", "application/dns-json")
        .send()?
        .json()?;

    let Some(answers) = dns.answer else {
        bail!("missing HTTPS record answers!");
    };
    let ech_b64: &str = answers[0]
        .data
        .split_whitespace()
        .find_map(|x| x.strip_prefix("ech="))
        .ok_or_else(|| anyhow::anyhow!("missing ECH config"))?;

    let ech_bytes: Vec<u8> = STANDARD.decode(ech_b64)?;

    let ech: EchConfig = EchConfig::new(
        ech_bytes.as_slice().into(),
        &[rustls::crypto::aws_lc_rs::hpke::DH_KEM_X25519_HKDF_SHA256_AES_128],
    )?;

    let provider: Arc<rustls::crypto::CryptoProvider> = Arc::new(aws_lc_rs::default_provider());
    let tls: ClientConfig = ClientConfig::builder_with_provider(provider)
        .with_ech(EchMode::Enable(ech))?
        .with_platform_verifier()?
        .with_no_client_auth();

    Ok(tls)
}
