use base64::{Engine, engine::general_purpose::STANDARD};
use md5::{Digest as _, Md5};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CryptoError {
    #[error("SSE-C requires algorithm, key, and key MD5 together")]
    IncompleteSseCustomerKey,
    #[error("unsupported SSE-C algorithm")]
    UnsupportedSseAlgorithm,
    #[error("invalid SSE-C key")]
    InvalidSseKey,
    #[error("SSE-C key MD5 mismatch")]
    SseKeyMd5Mismatch,
}

#[derive(Debug)]
pub struct SseKey {
    pub algorithm: String,
    pub key: String,
    pub key_md5: String,
}

pub fn select_sse_key(
    algorithm: Option<&str>,
    key: Option<&str>,
    key_md5: Option<&str>,
    platform_key: &str,
) -> Result<SseKey, CryptoError> {
    match (algorithm, key, key_md5) {
        (None, None, None) => build_sse_key("AES256", platform_key, None),
        (Some(algorithm), Some(key), Some(key_md5)) => build_sse_key(algorithm, key, Some(key_md5)),
        _ => Err(CryptoError::IncompleteSseCustomerKey),
    }
}

fn build_sse_key(
    algorithm: &str,
    key: &str,
    claimed_md5: Option<&str>,
) -> Result<SseKey, CryptoError> {
    if algorithm != "AES256" {
        return Err(CryptoError::UnsupportedSseAlgorithm);
    }
    let raw = STANDARD
        .decode(key)
        .map_err(|_| CryptoError::InvalidSseKey)?;
    if raw.len() != 32 {
        return Err(CryptoError::InvalidSseKey);
    }
    let computed_md5 = STANDARD.encode(Md5::digest(raw));
    if claimed_md5.is_some_and(|claimed| claimed != computed_md5) {
        return Err(CryptoError::SseKeyMd5Mismatch);
    }
    Ok(SseKey {
        algorithm: algorithm.into(),
        key: key.into(),
        key_md5: computed_md5,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_customer_key_or_fills_platform_key() {
        let platform_key = STANDARD.encode([1; 32]);
        let customer_key = STANDARD.encode([2; 32]);
        let customer_md5 = STANDARD.encode(Md5::digest([2; 32]));

        let filled = select_sse_key(None, None, None, &platform_key).unwrap();
        assert_eq!(filled.key, platform_key);
        assert!(
            select_sse_key(
                Some("AES256"),
                Some(&customer_key),
                Some(&customer_md5),
                &platform_key
            )
            .is_ok()
        );
        assert_eq!(
            select_sse_key(Some("AES256"), Some(&customer_key), None, &platform_key).unwrap_err(),
            CryptoError::IncompleteSseCustomerKey
        );
    }
}
