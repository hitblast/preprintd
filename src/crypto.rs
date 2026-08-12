use anyhow::Result;
use base64::prelude::*;
use rand::Rng;
use sha2::{Digest, Sha256};

use crate::WORKER_KEY;

pub fn decrypt(opt: Option<&str>, job_id: &str) -> Result<Vec<u8>> {
    let Some(value) = opt.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(Vec::new());
    };

    let raw = BASE64_STANDARD.decode(value)?;

    if raw.len() < 16 {
        return Ok(Vec::new());
    }

    let (iv, encrypted) = raw.split_at(16);

    let mut seed = Sha256::new();
    seed.update(WORKER_KEY.as_bytes());
    seed.update(iv);
    seed.update(job_id.as_bytes());
    let p = seed.finalize();

    let mut output = Vec::with_capacity(encrypted.len());

    for (idx, chunk) in encrypted.chunks(32).enumerate() {
        let mut hasher = Sha256::new();
        hasher.update(p);
        hasher.update((idx as u32).to_be_bytes());
        let key_stream = hasher.finalize();

        output.extend(
            chunk
                .iter()
                .zip(key_stream.iter())
                .map(|(byte, key)| byte ^ key),
        );
    }

    Ok(output)
}

#[allow(dead_code)]
pub fn encrypt(plaintext: &str, job_id: &str) -> Result<String> {
    let mut iv = [0u8; 16];
    rand::rng().fill_bytes(&mut iv);

    let mut seed = Sha256::new();
    seed.update(WORKER_KEY.as_bytes());
    seed.update(&iv);
    seed.update(job_id.as_bytes());
    let p = seed.finalize();

    let mut encrypted = Vec::with_capacity(plaintext.len());
    for (idx, chunk) in plaintext.as_bytes().chunks(32).enumerate() {
        let mut hasher = Sha256::new();
        hasher.update(p);
        hasher.update((idx as u32).to_be_bytes());
        let key_stream = hasher.finalize();
        encrypted.extend(
            chunk
                .iter()
                .zip(key_stream.iter())
                .map(|(byte, key)| byte ^ key),
        );
    }

    let mut raw = Vec::with_capacity(16 + encrypted.len());
    raw.extend_from_slice(&iv);
    raw.extend_from_slice(&encrypted);

    Ok(BASE64_STANDARD.encode(raw))
}
