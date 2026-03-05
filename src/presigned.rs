use hmac::{Hmac, Mac};
use rand::Rng;
use sha2::Sha256;

use crate::error::AppError;

type HmacSha256 = Hmac<Sha256>;

const UPLOAD_TOKEN_PREFIX: &str = "upload_";
const UPLOAD_TOKEN_SUFFIX_LEN: usize = 32;
const MEDIA_ID_HEX_LEN: usize = 16;
const RANDOM_SUFFIX_LEN: usize = UPLOAD_TOKEN_SUFFIX_LEN - MEDIA_ID_HEX_LEN;

// ---------------------------------------------------------------------------
// Download URL signing
// ---------------------------------------------------------------------------

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
    verify_hmac(&message, signature, signing_secret)
}

// ---------------------------------------------------------------------------
// Upload token encoding
// ---------------------------------------------------------------------------

/// Generates an opaque upload token encoding the given media ID.
///
/// Format: `upload_{hex(media_id)}{16 random alphanums}` (32 chars after prefix).
pub fn generate_upload_token(media_id: i64) -> String {
    let id_hex = hex::encode(media_id.to_be_bytes());
    let random_suffix: String = rand::rng()
        .sample_iter(&rand::distr::Alphanumeric)
        .take(RANDOM_SUFFIX_LEN)
        .map(char::from)
        .collect();
    format!("{UPLOAD_TOKEN_PREFIX}{id_hex}{random_suffix}")
}

/// Extracts the media ID encoded in an upload token.
pub fn decode_upload_token(token: &str) -> Result<i64, AppError> {
    let suffix = token
        .strip_prefix(UPLOAD_TOKEN_PREFIX)
        .ok_or_else(|| AppError::BadRequest("invalid upload token format".into()))?;
    if suffix.len() != UPLOAD_TOKEN_SUFFIX_LEN {
        return Err(AppError::BadRequest("invalid upload token length".into()));
    }
    let id_hex = &suffix[..MEDIA_ID_HEX_LEN];
    let bytes = hex::decode(id_hex)
        .map_err(|_| AppError::BadRequest("invalid upload token encoding".into()))?;
    let arr: [u8; 8] = bytes
        .try_into()
        .map_err(|_| AppError::BadRequest("invalid upload token".into()))?;
    Ok(i64::from_be_bytes(arr))
}

// ---------------------------------------------------------------------------
// Upload URL signing
// ---------------------------------------------------------------------------

/// Generates a signed upload URL path for the given token and parameters.
///
/// Returns `(url_path, expires_unix_timestamp)`.
pub fn sign_upload_url(
    token: &str,
    file_size: i64,
    content_type: &str,
    signing_secret: &str,
    ttl_secs: u64,
) -> (String, u64) {
    #[allow(clippy::cast_sign_loss)]
    let expires = chrono::Utc::now().timestamp() as u64 + ttl_secs;
    let message = format!("{token}:{expires}:{file_size}:{content_type}");
    let signature = sign(&message, signing_secret);
    let encoded_type = content_type.replace('/', "%2F");
    let url = format!(
        "/api/v1/media-management/media/upload/{token}\
         ?signature={signature}&expires={expires}&size={file_size}&type={encoded_type}"
    );
    (url, expires)
}

/// Verifies a signed upload URL's signature and expiry.
pub fn verify_upload_signature(
    token: &str,
    signature: &str,
    expires: u64,
    size: i64,
    content_type: &str,
    signing_secret: &str,
) -> Result<(), AppError> {
    #[allow(clippy::cast_sign_loss)]
    let now = chrono::Utc::now().timestamp() as u64;
    if now > expires {
        return Err(AppError::Unauthorized("upload URL has expired".into()));
    }

    let message = format!("{token}:{expires}:{size}:{content_type}");
    verify_hmac(&message, signature, signing_secret)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn sign(message: &str, secret: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(message.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

fn verify_hmac(message: &str, signature: &str, secret: &str) -> Result<(), AppError> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| AppError::Internal(format!("HMAC init failed: {e}")))?;
    mac.update(message.as_bytes());

    let sig_bytes = hex::decode(signature)
        .map_err(|_| AppError::Unauthorized("invalid signature encoding".into()))?;

    mac.verify_slice(&sig_bytes)
        .map_err(|_| AppError::Unauthorized("invalid signature".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "test-secret-key";

    // --- Download URL ---

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
        let params = parse_query_params(&url);

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
        let params = parse_query_params(&url);
        let expires: u64 = params["expires"].parse().unwrap();

        let result = verify_download_url(42, &"deadbeef".repeat(8), expires, TEST_SECRET);
        assert!(matches!(result, Err(AppError::Unauthorized(_))));
    }

    #[test]
    fn verify_rejects_wrong_media_id() {
        let url = generate_download_url(42, "complete", TEST_SECRET, 3600).unwrap();
        let params = parse_query_params(&url);

        let signature = params["signature"];
        let expires: u64 = params["expires"].parse().unwrap();

        let result = verify_download_url(99, signature, expires, TEST_SECRET);
        assert!(matches!(result, Err(AppError::Unauthorized(_))));
    }

    #[test]
    fn verify_rejects_wrong_secret() {
        let url = generate_download_url(42, "complete", TEST_SECRET, 3600).unwrap();
        let params = parse_query_params(&url);

        let signature = params["signature"];
        let expires: u64 = params["expires"].parse().unwrap();

        let result = verify_download_url(42, signature, expires, "wrong-secret");
        assert!(matches!(result, Err(AppError::Unauthorized(_))));
    }

    // --- Upload token ---

    #[test]
    fn upload_token_roundtrip() {
        let token = generate_upload_token(42);
        assert!(token.starts_with(UPLOAD_TOKEN_PREFIX));
        assert_eq!(
            token.len(),
            UPLOAD_TOKEN_PREFIX.len() + UPLOAD_TOKEN_SUFFIX_LEN
        );
        assert_eq!(decode_upload_token(&token).unwrap(), 42);
    }

    #[test]
    fn upload_token_roundtrip_large_id() {
        let token = generate_upload_token(i64::MAX);
        assert_eq!(decode_upload_token(&token).unwrap(), i64::MAX);
    }

    #[test]
    fn decode_upload_token_rejects_invalid_prefix() {
        let result = decode_upload_token("download_abcdef1234567890abcdef1234567890");
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[test]
    fn decode_upload_token_rejects_wrong_length() {
        let result = decode_upload_token("upload_abc");
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    // --- Upload URL signing ---

    #[test]
    fn sign_upload_url_returns_valid_url() {
        let token = generate_upload_token(42);
        let (url, expires) = sign_upload_url(&token, 1024, "image/jpeg", TEST_SECRET, 900);
        assert!(url.contains(&token));
        assert!(url.contains("signature="));
        assert!(url.contains(&format!("expires={expires}")));
        assert!(url.contains("size=1024"));
        assert!(url.contains("type=image%2Fjpeg"));
    }

    #[test]
    fn roundtrip_sign_then_verify_upload() {
        let token = generate_upload_token(42);
        let (url, _) = sign_upload_url(&token, 1024, "image/jpeg", TEST_SECRET, 900);
        let params = parse_query_params(&url);

        let signature = params["signature"];
        let expires: u64 = params["expires"].parse().unwrap();
        let size: i64 = params["size"].parse().unwrap();
        let content_type = params["type"].replace("%2F", "/");

        assert!(
            verify_upload_signature(&token, signature, expires, size, &content_type, TEST_SECRET)
                .is_ok()
        );
    }

    #[test]
    fn verify_upload_rejects_expired() {
        let token = generate_upload_token(42);
        let result = verify_upload_signature(&token, "abcd", 0, 1024, "image/jpeg", TEST_SECRET);
        assert!(matches!(result, Err(AppError::Unauthorized(_))));
    }

    #[test]
    fn verify_upload_rejects_tampered_signature() {
        let token = generate_upload_token(42);
        let (url, _) = sign_upload_url(&token, 1024, "image/jpeg", TEST_SECRET, 900);
        let params = parse_query_params(&url);
        let expires: u64 = params["expires"].parse().unwrap();

        let result = verify_upload_signature(
            &token,
            &"deadbeef".repeat(8),
            expires,
            1024,
            "image/jpeg",
            TEST_SECRET,
        );
        assert!(matches!(result, Err(AppError::Unauthorized(_))));
    }

    #[test]
    fn verify_upload_rejects_wrong_size() {
        let token = generate_upload_token(42);
        let (url, _) = sign_upload_url(&token, 1024, "image/jpeg", TEST_SECRET, 900);
        let params = parse_query_params(&url);

        let signature = params["signature"];
        let expires: u64 = params["expires"].parse().unwrap();

        let result =
            verify_upload_signature(&token, signature, expires, 2048, "image/jpeg", TEST_SECRET);
        assert!(matches!(result, Err(AppError::Unauthorized(_))));
    }

    #[test]
    fn verify_upload_rejects_wrong_content_type() {
        let token = generate_upload_token(42);
        let (url, _) = sign_upload_url(&token, 1024, "image/jpeg", TEST_SECRET, 900);
        let params = parse_query_params(&url);

        let signature = params["signature"];
        let expires: u64 = params["expires"].parse().unwrap();

        let result =
            verify_upload_signature(&token, signature, expires, 1024, "image/png", TEST_SECRET);
        assert!(matches!(result, Err(AppError::Unauthorized(_))));
    }

    // --- Test helpers ---

    fn parse_query_params(url: &str) -> std::collections::HashMap<&str, &str> {
        url.split_once('?')
            .unwrap()
            .1
            .split('&')
            .filter_map(|p| p.split_once('='))
            .collect()
    }
}
