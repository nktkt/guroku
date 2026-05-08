use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use guroku::integrity::{sha512_hex, verify};

fn hex_to_bytes(h: &str) -> Vec<u8> {
    (0..h.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&h[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn sha512_of_empty_input() {
    let expected = "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e";
    assert_eq!(sha512_hex(b""), expected);
}

#[test]
fn sha512_of_known_string_abc() {
    let expected = "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f";
    assert_eq!(sha512_hex(b"abc"), expected);
}

#[test]
fn sha512_hex_is_lowercase_and_128_chars() {
    let out = sha512_hex(b"the quick brown fox");
    assert_eq!(out.len(), 128);
    assert!(out
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
}

#[test]
fn verify_accepts_correct_integrity() {
    let data = b"hello world";
    let hex = sha512_hex(data);
    let digest = hex_to_bytes(&hex);
    let b64 = STANDARD.encode(&digest);
    let integrity = format!("sha512-{}", b64);
    let result = verify(data, &integrity, "pkg", "1.0.0");
    assert!(result.is_ok(), "expected Ok, got {:?}", result);
}

#[test]
fn verify_rejects_unsupported_algo() {
    let err = verify(b"x", "sha1-deadbeef", "pkg", "1.0").unwrap_err();
    let msg = format!("{:?}", err);
    assert!(
        msg.contains("UnsupportedIntegrity"),
        "expected UnsupportedIntegrity, got {:?}",
        err
    );
}
