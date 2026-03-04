use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::error::AppError;

type HmacSha256 = Hmac<Sha256>;

/// Generates a signed download URL path for the given media ID.
///
/// Returns `None` if `processing_status` is not `"complete"`.
pub fn generate_download_url(
    media_id: i64,
    processing_status: &str,
    signing_secret: &str,
    ttl_secs: u64,
) -> Option<String> {
    if processing_status != "complete" {
        return None;
    }
    #[allow(clippy::cast_sign_loss)]
    let expires = chrono::Utc::now().timestamp() as u64 + ttl_secs;
    let message = format!("{media_id}:{expires}");
    let signature = sign(&message, signing_secret);
    Some(format!(
        "/api/v1/media-management/media/{media_id}/download\
         ?signature={signature}&expires={expires}"
    ))
}

/// Verifies a signed download URL's signature and expiry.
pub fn verify_download_url(
    media_id: i64,
    signature: &str,
    expires: u64,
    signing_secret: &str,
) -> Result<(), AppError> {
    #[allow(clippy::cast_sign_loss)]
    let now = chrono::Utc::now().timestamp() as u64;
    if now > expires {
        return Err(AppError::Unauthorized("download URL has expired".into()));
    }

    let message = format!("{media_id}:{expires}");
    let mut mac = HmacSha256::new_from_slice(signing_secret.as_bytes())
        .map_err(|e| AppError::Internal(format!("HMAC init failed: {e}")))?;
    mac.update(message.as_bytes());

    let sig_bytes = hex::decode(signature)
        .map_err(|_| AppError::Unauthorized("invalid signature encoding".into()))?;

    mac.verify_slice(&sig_bytes)
        .map_err(|_| AppError::Unauthorized("invalid signature".into()))
}

fn sign(message: &str, secret: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(message.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "test-secret-key";

    #[test]
    fn generate_returns_none_for_non_complete_status() {
        assert!(generate_download_url(1, "pending", TEST_SECRET, 3600).is_none());
        assert!(generate_download_url(1, "processing", TEST_SECRET, 3600).is_none());
        assert!(generate_download_url(1, "failed", TEST_SECRET, 3600).is_none());
    }

    #[test]
    fn generate_returns_url_for_complete_status() {
        let url = generate_download_url(42, "complete", TEST_SECRET, 3600);
        assert!(url.is_some());
        let url = url.unwrap();
        assert!(url.starts_with("/api/v1/media-management/media/42/download?"));
        assert!(url.contains("signature="));
        assert!(url.contains("expires="));
    }

    #[test]
    fn roundtrip_generate_then_verify() {
        let url = generate_download_url(42, "complete", TEST_SECRET, 3600).unwrap();
        let params: std::collections::HashMap<&str, &str> = url
            .split_once('?')
            .unwrap()
            .1
            .split('&')
            .filter_map(|p| p.split_once('='))
            .collect();

        let signature = params["signature"];
        let expires: u64 = params["expires"].parse().unwrap();

        assert!(verify_download_url(42, signature, expires, TEST_SECRET).is_ok());
    }

    #[test]
    fn verify_rejects_expired_url() {
        let result = verify_download_url(42, "abcd", 0, TEST_SECRET);
        assert!(matches!(result, Err(AppError::Unauthorized(_))));
    }

    #[test]
    fn verify_rejects_tampered_signature() {
        let url = generate_download_url(42, "complete", TEST_SECRET, 3600).unwrap();
        let params: std::collections::HashMap<&str, &str> = url
            .split_once('?')
            .unwrap()
            .1
            .split('&')
            .filter_map(|p| p.split_once('='))
            .collect();

        let expires: u64 = params["expires"].parse().unwrap();

        let result = verify_download_url(42, "deadbeef".repeat(8).as_str(), expires, TEST_SECRET);
        assert!(matches!(result, Err(AppError::Unauthorized(_))));
    }

    #[test]
    fn verify_rejects_wrong_media_id() {
        let url = generate_download_url(42, "complete", TEST_SECRET, 3600).unwrap();
        let params: std::collections::HashMap<&str, &str> = url
            .split_once('?')
            .unwrap()
            .1
            .split('&')
            .filter_map(|p| p.split_once('='))
            .collect();

        let signature = params["signature"];
        let expires: u64 = params["expires"].parse().unwrap();

        let result = verify_download_url(99, signature, expires, TEST_SECRET);
        assert!(matches!(result, Err(AppError::Unauthorized(_))));
    }

    #[test]
    fn verify_rejects_wrong_secret() {
        let url = generate_download_url(42, "complete", TEST_SECRET, 3600).unwrap();
        let params: std::collections::HashMap<&str, &str> = url
            .split_once('?')
            .unwrap()
            .1
            .split('&')
            .filter_map(|p| p.split_once('='))
            .collect();

        let signature = params["signature"];
        let expires: u64 = params["expires"].parse().unwrap();

        let result = verify_download_url(42, signature, expires, "wrong-secret");
        assert!(matches!(result, Err(AppError::Unauthorized(_))));
    }
}
