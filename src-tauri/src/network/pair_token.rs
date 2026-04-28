//! Pairing-payload codec — Chunk 24.2a.
//!
//! Pure utility for the Phase 24 mobile-companion flow. Encodes the
//! handshake parameters the desktop needs to deliver to a phone over
//! the air-gap (printed QR / NFC tap / share-sheet) into a stable
//! `terransoul://pair?...` URI, and decodes the same URI back into a
//! validated [`PairPayload`].
//!
//! The codec is *pure* — no I/O, no networking, no clock except via
//! caller-supplied `now_unix_ms` for expiry checks — so every rule is
//! deterministically unit-testable. Random-token *generation* is the
//! only OS-coupled function, deliberately isolated to one call so the
//! rest of the module is fully sandboxable.
//!
//! ## What 24.2a ships
//!
//! - [`PairPayload`] — the on-the-wire handshake struct.
//! - [`encode_uri`] / [`decode_uri`] — round-trippable codec.
//! - [`gen_token`] — 32-byte cryptographically-random pairing token
//!   (via `rand_core::OsRng`).
//! - [`constant_time_eq`] — fixed-time byte-slice comparison for
//!   token verification (hand-rolled — no `subtle` dep).
//! - [`is_expired`] — replay-protection helper that checks
//!   `payload.expires_at_unix_ms` against a caller-supplied clock.
//! - [`DEFAULT_EXPIRY_MS`] — 5-minute pairing window.
//!
//! ## What 24.2a does *not* ship (deferred to 24.2b)
//!
//! - Database persistence (`paired_devices` SQLite table).
//! - Self-signed CA generation via `rcgen`.
//! - The Tauri commands (`start_pairing`, `confirm_pairing`,
//!   `revoke_device`, `list_paired_devices`).
//! - The actual server-side pairing window enforcement.
//!
//! All of those compose `PairPayload` + `gen_token` + `is_expired`
//! without re-engineering anything.

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Default pairing-window length: 5 minutes from issuance.
pub const DEFAULT_EXPIRY_MS: u64 = 5 * 60 * 1000;

/// Length of a pairing token, in bytes. 32 = 256-bit security level —
/// matches the conservative posture for short-lived secrets.
pub const TOKEN_BYTE_LEN: usize = 32;

/// Maximum length of the encoded URI, in characters. QR codes degrade
/// in scannability past ~512 chars; we cap below that to leave room
/// for an optional display-name extension in 24.2b.
pub const MAX_URI_LEN: usize = 480;

/// URI scheme advertised by the desktop and recognised by the iOS app.
pub const PAIR_URI_SCHEME: &str = "terransoul";

/// URI host segment (after the scheme) — fixed string.
pub const PAIR_URI_HOST: &str = "pair";

/// On-the-wire handshake payload. Everything the phone needs to open
/// the gRPC channel and validate the certificate fingerprint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairPayload {
    /// LAN address of the desktop (e.g. `192.168.1.42`).
    pub host: String,
    /// gRPC server port on the desktop.
    pub port: u16,
    /// 32-byte pairing token, base64url-no-pad encoded for transport.
    pub token_b64: String,
    /// SHA-256 of the desktop's TLS certificate (DER form), base64url-no-pad.
    pub fingerprint_b64: String,
    /// Unix-millisecond timestamp after which the payload is invalid.
    pub expires_at_unix_ms: u64,
}

/// Codec / validation errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairError {
    /// URI scheme is not `terransoul://`.
    BadScheme,
    /// URI host is not `pair`.
    BadHost,
    /// URI is missing a required query parameter.
    MissingField(&'static str),
    /// A required query parameter could not be parsed.
    InvalidField(&'static str),
    /// The encoded URI exceeds [`MAX_URI_LEN`].
    UriTooLong,
    /// The decoded token / fingerprint is not the expected length.
    BadByteLength {
        field: &'static str,
        expected: usize,
        actual: usize,
    },
    /// Generic parse failure (URL parser).
    Malformed,
}

impl fmt::Display for PairError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PairError::BadScheme => write!(f, "URI scheme must be `terransoul://`"),
            PairError::BadHost => write!(f, "URI host must be `pair`"),
            PairError::MissingField(n) => write!(f, "missing required field: {n}"),
            PairError::InvalidField(n) => write!(f, "invalid value for field: {n}"),
            PairError::UriTooLong => {
                write!(f, "URI exceeds {} chars (QR-scan limit)", MAX_URI_LEN)
            }
            PairError::BadByteLength { field, expected, actual } => write!(
                f,
                "field `{field}` decoded to {actual} bytes, expected {expected}"
            ),
            PairError::Malformed => write!(f, "URI could not be parsed"),
        }
    }
}

impl std::error::Error for PairError {}

impl PairPayload {
    /// Construct from raw token + fingerprint bytes — encodes both as
    /// base64url-no-pad. Returns `Err(BadByteLength)` if the token is
    /// not exactly [`TOKEN_BYTE_LEN`] bytes.
    pub fn from_bytes(
        host: impl Into<String>,
        port: u16,
        token: &[u8],
        fingerprint: &[u8],
        expires_at_unix_ms: u64,
    ) -> Result<Self, PairError> {
        if token.len() != TOKEN_BYTE_LEN {
            return Err(PairError::BadByteLength {
                field: "token",
                expected: TOKEN_BYTE_LEN,
                actual: token.len(),
            });
        }
        // Fingerprint must look like a SHA-256 — exactly 32 bytes.
        if fingerprint.len() != 32 {
            return Err(PairError::BadByteLength {
                field: "fingerprint",
                expected: 32,
                actual: fingerprint.len(),
            });
        }
        Ok(Self {
            host: host.into(),
            port,
            token_b64: URL_SAFE_NO_PAD.encode(token),
            fingerprint_b64: URL_SAFE_NO_PAD.encode(fingerprint),
            expires_at_unix_ms,
        })
    }

    /// Decode the payload's token back to raw bytes.
    pub fn token_bytes(&self) -> Result<Vec<u8>, PairError> {
        let bytes = URL_SAFE_NO_PAD
            .decode(&self.token_b64)
            .map_err(|_| PairError::InvalidField("token"))?;
        if bytes.len() != TOKEN_BYTE_LEN {
            return Err(PairError::BadByteLength {
                field: "token",
                expected: TOKEN_BYTE_LEN,
                actual: bytes.len(),
            });
        }
        Ok(bytes)
    }

    /// Decode the payload's fingerprint back to raw bytes.
    pub fn fingerprint_bytes(&self) -> Result<Vec<u8>, PairError> {
        let bytes = URL_SAFE_NO_PAD
            .decode(&self.fingerprint_b64)
            .map_err(|_| PairError::InvalidField("fingerprint"))?;
        if bytes.len() != 32 {
            return Err(PairError::BadByteLength {
                field: "fingerprint",
                expected: 32,
                actual: bytes.len(),
            });
        }
        Ok(bytes)
    }
}

/// Encode a payload into a `terransoul://pair?...` URI suitable for
/// QR.
///
/// Returns `Err(UriTooLong)` when the rendered URI exceeds
/// [`MAX_URI_LEN`] chars — signals the caller that one of the
/// fields is too long for QR transport (almost always `host`).
pub fn encode_uri(payload: &PairPayload) -> Result<String, PairError> {
    // Build query string by hand — the field set is fixed and small,
    // so a full `url::Url::set_query_pairs` round-trip is overkill.
    let host = url_encode_component(&payload.host);
    let token = url_encode_component(&payload.token_b64);
    let fp = url_encode_component(&payload.fingerprint_b64);
    let uri = format!(
        "{scheme}://{host_seg}?host={host}&port={port}&token={token}&fp={fp}&exp={exp}",
        scheme = PAIR_URI_SCHEME,
        host_seg = PAIR_URI_HOST,
        host = host,
        port = payload.port,
        token = token,
        fp = fp,
        exp = payload.expires_at_unix_ms,
    );
    if uri.len() > MAX_URI_LEN {
        return Err(PairError::UriTooLong);
    }
    Ok(uri)
}

/// Decode a URI back to a [`PairPayload`].
///
/// Validates scheme, host, presence of every required field, and
/// the byte-length of the decoded token / fingerprint. Does **not**
/// check expiry — pass to [`is_expired`] separately so the caller
/// can decide whether to surface "expired" as a UX state vs reject
/// outright.
pub fn decode_uri(input: &str) -> Result<PairPayload, PairError> {
    if input.len() > MAX_URI_LEN {
        return Err(PairError::UriTooLong);
    }
    let parsed = url::Url::parse(input).map_err(|_| PairError::Malformed)?;
    if parsed.scheme() != PAIR_URI_SCHEME {
        return Err(PairError::BadScheme);
    }
    if parsed.host_str() != Some(PAIR_URI_HOST) {
        return Err(PairError::BadHost);
    }

    let mut host: Option<String> = None;
    let mut port: Option<u16> = None;
    let mut token_b64: Option<String> = None;
    let mut fp_b64: Option<String> = None;
    let mut exp: Option<u64> = None;

    for (k, v) in parsed.query_pairs() {
        match k.as_ref() {
            "host" => host = Some(v.into_owned()),
            "port" => {
                port = Some(
                    v.parse::<u16>()
                        .map_err(|_| PairError::InvalidField("port"))?,
                )
            }
            "token" => token_b64 = Some(v.into_owned()),
            "fp" => fp_b64 = Some(v.into_owned()),
            "exp" => {
                exp = Some(
                    v.parse::<u64>()
                        .map_err(|_| PairError::InvalidField("exp"))?,
                )
            }
            _ => { /* tolerate unknown extension keys */ }
        }
    }

    let host = host.ok_or(PairError::MissingField("host"))?;
    if host.is_empty() {
        return Err(PairError::InvalidField("host"));
    }
    let port = port.ok_or(PairError::MissingField("port"))?;
    let token_b64 = token_b64.ok_or(PairError::MissingField("token"))?;
    let fingerprint_b64 = fp_b64.ok_or(PairError::MissingField("fp"))?;
    let expires_at_unix_ms = exp.ok_or(PairError::MissingField("exp"))?;

    let payload = PairPayload {
        host,
        port,
        token_b64,
        fingerprint_b64,
        expires_at_unix_ms,
    };

    // Validate the byte-lengths so the caller doesn't have to.
    let _ = payload.token_bytes()?;
    let _ = payload.fingerprint_bytes()?;

    Ok(payload)
}

/// Generate a fresh 32-byte pairing token via the OS RNG.
///
/// This is the *only* impure function in this module and is
/// deliberately tiny so the rest of the module remains
/// deterministically testable.
pub fn gen_token() -> [u8; TOKEN_BYTE_LEN] {
    let mut buf = [0u8; TOKEN_BYTE_LEN];
    OsRng.fill_bytes(&mut buf);
    buf
}

/// Constant-time byte-slice equality.
///
/// Returns `false` immediately when lengths differ (length leaks are
/// fine — the *content* must not). Inside, every byte is XORed and
/// the `OR` accumulator is checked once at the end so the timing is
/// independent of where the first mismatch occurs.
///
/// The `black_box` calls discourage the optimiser from short-circuit
/// returning. Hand-rolled rather than pulling in the `subtle` crate
/// because the attack surface is one byte-comparison call site.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut acc: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        acc |= x ^ y;
    }
    std::hint::black_box(acc) == 0
}

/// Returns `true` when `now_unix_ms >= payload.expires_at_unix_ms`.
pub fn is_expired(payload: &PairPayload, now_unix_ms: u64) -> bool {
    now_unix_ms >= payload.expires_at_unix_ms
}

/// Minimal URL component encoder — only the chars that break the
/// query syntax (`&`, `=`, `#`, `?`, ` `, `+`, `%`). base64url-no-pad
/// strings are already URL-safe so the typical case is a no-op.
fn url_encode_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        match b {
            b'&' | b'=' | b'#' | b'?' | b' ' | b'+' | b'%' => {
                out.push('%');
                out.push_str(&format!("{:02X}", b));
            }
            _ => out.push(b as char),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_payload() -> PairPayload {
        PairPayload::from_bytes(
            "192.168.1.42",
            7422,
            &[0xAB; TOKEN_BYTE_LEN],
            &[0xCD; 32],
            1_700_000_000_000,
        )
        .unwrap()
    }

    #[test]
    fn from_bytes_rejects_short_token() {
        let err = PairPayload::from_bytes("h", 1, &[0u8; 31], &[0u8; 32], 0).unwrap_err();
        assert!(matches!(err, PairError::BadByteLength { field: "token", .. }));
    }

    #[test]
    fn from_bytes_rejects_short_fingerprint() {
        let err = PairPayload::from_bytes(
            "h",
            1,
            &[0u8; TOKEN_BYTE_LEN],
            &[0u8; 31],
            0,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            PairError::BadByteLength { field: "fingerprint", .. }
        ));
    }

    #[test]
    fn round_trip_encode_decode() {
        let original = sample_payload();
        let uri = encode_uri(&original).unwrap();
        let decoded = decode_uri(&uri).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn encoded_uri_uses_terransoul_pair_scheme() {
        let uri = encode_uri(&sample_payload()).unwrap();
        assert!(uri.starts_with("terransoul://pair?"), "got `{uri}`");
    }

    #[test]
    fn token_bytes_round_trip() {
        let payload = sample_payload();
        let bytes = payload.token_bytes().unwrap();
        assert_eq!(bytes, vec![0xAB; TOKEN_BYTE_LEN]);
    }

    #[test]
    fn fingerprint_bytes_round_trip() {
        let payload = sample_payload();
        let bytes = payload.fingerprint_bytes().unwrap();
        assert_eq!(bytes, vec![0xCD; 32]);
    }

    #[test]
    fn decode_rejects_bad_scheme() {
        let err = decode_uri("https://pair?host=h&port=1&token=a&fp=b&exp=0").unwrap_err();
        assert_eq!(err, PairError::BadScheme);
    }

    #[test]
    fn decode_rejects_bad_host() {
        let err =
            decode_uri("terransoul://other?host=h&port=1&token=a&fp=b&exp=0").unwrap_err();
        assert_eq!(err, PairError::BadHost);
    }

    #[test]
    fn decode_reports_each_missing_field() {
        // Build URIs that omit one field at a time, taking care that
        // the remaining query string is well-formed (no leading or
        // doubled `&`).
        let p = sample_payload();
        let parts: [(&str, String); 5] = [
            ("host", format!("host={}", p.host)),
            ("port", format!("port={}", p.port)),
            ("token", format!("token={}", p.token_b64)),
            ("fp", format!("fp={}", p.fingerprint_b64)),
            ("exp", format!("exp={}", p.expires_at_unix_ms)),
        ];
        for skip_idx in 0..parts.len() {
            let expected = parts[skip_idx].0;
            let query = parts
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != skip_idx)
                .map(|(_, (_, kv))| kv.as_str())
                .collect::<Vec<_>>()
                .join("&");
            let uri = format!("terransoul://pair?{query}");
            let err = decode_uri(&uri).unwrap_err();
            assert!(
                matches!(err, PairError::MissingField(f) if f == expected),
                "expected MissingField({expected}) got {err:?} for `{uri}`"
            );
        }
    }

    #[test]
    fn decode_rejects_unparseable_port() {
        let uri = "terransoul://pair?host=h&port=notanumber&token=AAA&fp=BBB&exp=0";
        assert_eq!(
            decode_uri(uri).unwrap_err(),
            PairError::InvalidField("port")
        );
    }

    #[test]
    fn decode_rejects_unparseable_exp() {
        let uri = "terransoul://pair?host=h&port=1&token=AAA&fp=BBB&exp=oops";
        assert_eq!(
            decode_uri(uri).unwrap_err(),
            PairError::InvalidField("exp")
        );
    }

    #[test]
    fn decode_rejects_short_token_byte_length() {
        // base64url("AA") = 1 byte, not 32.
        let uri = format!(
            "terransoul://pair?host=h&port=1&token=AA&fp={}&exp=0",
            URL_SAFE_NO_PAD.encode([0u8; 32])
        );
        let err = decode_uri(&uri).unwrap_err();
        assert!(matches!(err, PairError::BadByteLength { field: "token", .. }));
    }

    #[test]
    fn decode_tolerates_unknown_extension_keys() {
        let mut uri = encode_uri(&sample_payload()).unwrap();
        uri.push_str("&futurefield=hello");
        let decoded = decode_uri(&uri).unwrap();
        assert_eq!(decoded, sample_payload());
    }

    #[test]
    fn decode_rejects_uri_too_long() {
        let long_host = "a".repeat(MAX_URI_LEN + 100);
        let uri = format!(
            "terransoul://pair?host={long_host}&port=1&token=AAA&fp=BBB&exp=0"
        );
        assert_eq!(decode_uri(&uri).unwrap_err(), PairError::UriTooLong);
    }

    #[test]
    fn encode_rejects_uri_too_long() {
        let mut payload = sample_payload();
        payload.host = "h".repeat(MAX_URI_LEN);
        assert_eq!(encode_uri(&payload).unwrap_err(), PairError::UriTooLong);
    }

    #[test]
    fn gen_token_is_well_formed_and_nondeterministic() {
        let a = gen_token();
        let b = gen_token();
        assert_eq!(a.len(), TOKEN_BYTE_LEN);
        // Two consecutive 256-bit OS-random tokens must not collide.
        assert_ne!(a, b);
    }

    #[test]
    fn constant_time_eq_handles_length_mismatch() {
        assert!(!constant_time_eq(b"abc", b"ab"));
        assert!(!constant_time_eq(b"", b"a"));
    }

    #[test]
    fn constant_time_eq_matches_eq_on_content() {
        assert!(constant_time_eq(b"hello", b"hello"));
        assert!(!constant_time_eq(b"hello", b"hellp"));
        assert!(constant_time_eq(b"", b""));
    }

    #[test]
    fn is_expired_strict_ge_boundary() {
        let payload = PairPayload {
            expires_at_unix_ms: 100,
            ..sample_payload()
        };
        assert!(!is_expired(&payload, 99));
        // Exactly at the boundary is considered expired (cap is exclusive).
        assert!(is_expired(&payload, 100));
        assert!(is_expired(&payload, 101));
    }

    #[test]
    fn url_encode_component_passes_safe_chars() {
        // base64url-no-pad characters are all safe — encode_component
        // must be a no-op.
        let s = "AbCdEf-_0123";
        assert_eq!(url_encode_component(s), s);
    }

    #[test]
    fn url_encode_component_escapes_query_breakers() {
        let s = "a&b=c#d?e f+g%h";
        let encoded = url_encode_component(s);
        // Round-trip through the URL parser to confirm.
        let uri = format!("http://x/?v={encoded}");
        let parsed = url::Url::parse(&uri).unwrap();
        let v = parsed.query_pairs().find(|(k, _)| k == "v").unwrap().1;
        assert_eq!(v, s);
    }

    #[test]
    fn host_with_special_chars_round_trips() {
        // IPv6 host literal contains `:` which is reserved but not in
        // our breaker list — base64url-safe; should round-trip via
        // the URL crate's query parsing.
        let mut payload = sample_payload();
        payload.host = "fe80::1".to_string();
        let uri = encode_uri(&payload).unwrap();
        let decoded = decode_uri(&uri).unwrap();
        assert_eq!(decoded.host, "fe80::1");
    }

    #[test]
    fn default_expiry_is_five_minutes() {
        assert_eq!(DEFAULT_EXPIRY_MS, 300_000);
    }
}
