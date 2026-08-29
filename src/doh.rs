use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use reqwest::blocking::Client;
use reqwest::dns::{Addrs, Resolve, Resolving};
use rustls::client::{ClientConfig, EchConfig, EchMode};
use rustls::crypto::aws_lc_rs;
use rustls_platform_verifier::BuilderVerifierExt;

const QUERY_URL: &str = "https://1.1.1.1/dns-query";

#[derive(Debug, serde::Deserialize)]
struct DohResponse {
    #[serde(rename = "Answer")]
    answer: Option<Vec<DohAnswer>>,
}

#[derive(Debug, serde::Deserialize)]
struct DohAnswer {
    #[serde(rename = "type")]
    record_type: u16,
    #[serde(rename = "TTL")]
    ttl: u64,
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

#[derive(PartialEq)]
struct DohCacheObject {
    stored_at: std::time::Instant,
    addrs: Vec<SocketAddr>,
    min_ttl: u64,
}

static DOH_CACHE: LazyLock<Mutex<HashMap<String, DohCacheObject>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

impl Resolve for DohResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> Resolving {
        let host = name.as_str().to_owned();

        let cached = {
            let mut cache = DOH_CACHE.lock().expect("DOH cache lock poisoned");

            let expired = cache.get(&host).is_some_and(|f| {
                Instant::now().saturating_duration_since(f.stored_at)
                    <= Duration::from_secs(f.min_ttl)
            });

            if expired {
                cache.remove(&host);
                None
            } else {
                cache.get(&host).map(|f| f.addrs.clone())
            }
        };

        if let Some(addrs) = cached {
            return Box::pin(async move { Ok(Box::new(addrs.into_iter()) as Addrs) });
        }

        let dns: DohResponse = match DOH_CLIENT
            .get(format!("{QUERY_URL}?name={host}&type=A"))
            .header("Accept", "application/dns-json")
            .send()
        {
            Ok(response) => match response.json() {
                Ok(dns) => dns,
                Err(e) => return Box::pin(async move { Err(Box::new(e) as _) }),
            },
            Err(e) => return Box::pin(async move { Err(Box::new(e) as _) }),
        };

        let min_ttl = dns.answer.as_ref().and_then(|f| {
            f.iter()
                .filter(|f| f.record_type == 1)
                .map(|answer| answer.ttl)
                .min()
        });

        let addrs: Vec<SocketAddr> = dns
            .answer
            .unwrap_or_default()
            .into_iter()
            .filter(|record| record.record_type == 1)
            .filter_map(|record| record.data.parse::<IpAddr>().ok().map(|ip| (ip, 0).into()))
            .collect();

        if !addrs.is_empty()
            && let Some(min_ttl) = min_ttl
        {
            let mut cache = DOH_CACHE.lock().expect("DOH cache lock poisoned");

            cache.insert(
                host,
                DohCacheObject {
                    stored_at: Instant::now(),
                    addrs: addrs.clone(),
                    min_ttl,
                },
            );
        }

        Box::pin(async move { Ok(Box::new(addrs.into_iter()) as Addrs) })
    }
}

pub fn ready_tls_config(domain: &str) -> Result<ClientConfig> {
    let dns: DohResponse = DOH_CLIENT
        .get(format!("{QUERY_URL}?name={domain}&type=HTTPS"))
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
