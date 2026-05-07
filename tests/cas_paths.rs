use guroku::cache::{cas_dir, cas_entry, home, safe_segment};

const SHA512_HEX: &str = "e1b85b27d6bcb05846c18e6a48f118e89f0c0587b1a16d3a3a83c1c0f4a2b8d9\
                         c2e3f1a0b4c5d6e7f8091a2b3c4d5e6f70819a2b3c4d5e6f7081920a1b2c3d4e";

#[test]
fn cas_dir_is_under_home() {
    let cas = cas_dir().unwrap();
    let h = home().unwrap();
    assert!(
        cas.starts_with(&h),
        "cas_dir {:?} should start with home {:?}",
        cas,
        h
    );
}

#[test]
fn cas_dir_ends_with_cas() {
    let cas = cas_dir().unwrap();
    assert_eq!(
        cas.file_name().and_then(|s| s.to_str()),
        Some("cas"),
        "last component of {:?} should be 'cas'",
        cas
    );
}

#[test]
fn cas_entry_uses_two_char_prefix() {
    assert_eq!(SHA512_HEX.len(), 128, "test fixture must be 128 hex chars");
    let entry = cas_entry(SHA512_HEX).unwrap();
    let comps: Vec<String> = entry
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    let prefix = &SHA512_HEX[..2];
    let rest = &SHA512_HEX[2..];
    assert!(
        comps.iter().any(|c| c == prefix),
        "expected component {:?} in {:?}",
        prefix,
        comps
    );
    assert!(
        comps.iter().any(|c| c == rest),
        "expected component {:?} in {:?}",
        rest,
        comps
    );
}

#[test]
fn cas_entry_rejects_short_hex() {
    let result = cas_entry("abc");
    assert!(
        result.is_err(),
        "expected Err for short hex, got {:?}",
        result
    );
}

#[test]
fn cas_entry_under_cas_dir() {
    let entry = cas_entry(SHA512_HEX).unwrap();
    let cas = cas_dir().unwrap();
    assert!(
        entry.starts_with(&cas),
        "entry {:?} should be under cas_dir {:?}",
        entry,
        cas
    );
}

#[test]
fn safe_segment_replaces_slash() {
    assert_eq!(safe_segment("@types/node"), "@types+node");
}

#[test]
fn safe_segment_passes_unscoped_through() {
    assert_eq!(safe_segment("lodash"), "lodash");
}
