use base64::Engine;
use sha2::{Digest, Sha512};

use guroku::error::GurokuError;
use guroku::integrity::{sha512_hex, verify};

fn b64_sha512(bytes: &[u8]) -> String {
    let mut hasher = Sha512::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    base64::engine::general_purpose::STANDARD.encode(digest)
}

#[test]
fn verifies_correct_sha512() {
    let bytes = b"hello world";
    let encoded = b64_sha512(bytes);
    let integrity = format!("sha512-{}", encoded);
    let result = verify(bytes, &integrity, "pkg", "1.0.0");
    assert!(result.is_ok(), "expected Ok, got {:?}", result);
}

#[test]
fn rejects_wrong_sha512() {
    let bytes = b"hello world";
    let other_encoded = b64_sha512(b"goodbye world");
    let integrity = format!("sha512-{}", other_encoded);
    let result = verify(bytes, &integrity, "pkg", "1.0.0");
    match result {
        Err(GurokuError::IntegrityMismatch { .. }) => {}
        other => panic!("expected IntegrityMismatch, got {:?}", other),
    }
}

#[test]
fn sha512_hex_is_lowercase_128_chars() {
    let hex = sha512_hex(b"hello world");
    assert_eq!(hex.len(), 128, "expected 128 chars, got {}", hex.len());
    assert!(
        hex.chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
        "expected lowercase hex, got {}",
        hex
    );
}

#[test]
fn verify_returns_invalid_for_malformed_integrity() {
    let bytes = b"hello world";
    let result = verify(bytes, "sha512nodash", "pkg", "1.0.0");
    assert!(result.is_err(), "expected Err for malformed integrity");
}
