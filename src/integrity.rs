use crate::error::{GurokuError, Result};
use base64::Engine;
use sha2::{Digest, Sha512};

/// Verify an npm-style `integrity` string against the bytes of a tarball.
///
/// Format: `<algo>-<base64(digest)>`. Only `sha512` is supported in v0.1.
pub fn verify(bytes: &[u8], integrity: &str, name: &str, version: &str) -> Result<()> {
    let (algo, encoded) = integrity
        .split_once('-')
        .ok_or_else(|| GurokuError::InvalidIntegrity(integrity.to_string()))?;

    match algo {
        "sha512" => verify_sha512(bytes, encoded, name, version),
        other => Err(GurokuError::UnsupportedIntegrity(other.to_string())),
    }
}

pub fn verify_sha512(bytes: &[u8], encoded: &str, name: &str, version: &str) -> Result<()> {
    let expected = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| GurokuError::InvalidIntegrity(encoded.to_string()))?;

    let mut hasher = Sha512::new();
    hasher.update(bytes);
    let actual = hasher.finalize();

    if actual.as_slice() == expected.as_slice() {
        Ok(())
    } else {
        Err(GurokuError::IntegrityMismatch {
            name: name.to_string(),
            version: version.to_string(),
            detail: "sha512 mismatch".into(),
        })
    }
}

pub fn sha512_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha512::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}
