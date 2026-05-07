use guroku::error::GurokuError;
use guroku::integrity::verify;

#[test]
fn rejects_sha1_algorithm() {
    let result = verify(b"x", "sha1-abcdef", "pkg", "1.0.0");
    assert!(matches!(result, Err(GurokuError::UnsupportedIntegrity(_))));
}

#[test]
fn rejects_sha256_algorithm() {
    let result = verify(b"x", "sha256-abcdef", "pkg", "1.0.0");
    assert!(matches!(result, Err(GurokuError::UnsupportedIntegrity(_))));
}

#[test]
fn rejects_invalid_format_no_dash() {
    let result = verify(b"x", "noseparator", "pkg", "1.0.0");
    assert!(matches!(result, Err(GurokuError::InvalidIntegrity(_))));
}

#[test]
fn rejects_invalid_base64() {
    let result = verify(b"x", "sha512-not_valid_base64!@#", "pkg", "1.0.0");
    assert!(result.is_err());
}
