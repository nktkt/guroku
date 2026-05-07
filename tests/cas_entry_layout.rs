use guroku::cache::{cas_dir, cas_entry};

fn hex128() -> String {
    let s = "abcdef0123".repeat(13);
    s[..128].to_string()
}

fn hex128_other() -> String {
    let s = "0123456789".repeat(13);
    s[..128].to_string()
}

#[test]
fn cas_entry_distinct_hashes_yield_distinct_paths() {
    let a = hex128();
    let b = hex128_other();
    assert_ne!(a, b, "fixtures must differ");
    let pa = cas_entry(&a).unwrap();
    let pb = cas_entry(&b).unwrap();
    assert_ne!(pa, pb, "distinct hashes should yield distinct paths");
}

#[test]
fn cas_entry_same_hash_yields_same_path() {
    let h = hex128();
    let p1 = cas_entry(&h).unwrap();
    let p2 = cas_entry(&h).unwrap();
    assert_eq!(p1, p2, "same hash must yield same path");
}

#[test]
fn cas_entry_two_char_prefix_dir() {
    let h = hex128();
    assert!(h.starts_with("ab"));
    let entry = cas_entry(&h).unwrap();
    let parent = entry.parent().expect("entry should have a parent");
    assert_eq!(
        parent.file_name().and_then(|s| s.to_str()),
        Some("ab"),
        "parent dir name should be the 2-char prefix"
    );
}

#[test]
fn cas_entry_leaf_is_remaining_hex() {
    let h = hex128();
    let entry = cas_entry(&h).unwrap();
    let leaf = entry
        .file_name()
        .and_then(|s| s.to_str())
        .expect("entry should have a file name");
    let expected = &h[2..];
    assert_eq!(leaf.len(), 126, "leaf should be 126 chars");
    assert_eq!(leaf, expected, "leaf should equal hex[2..]");
}

#[test]
fn cas_entry_under_cas_dir() {
    let h = hex128();
    let entry = cas_entry(&h).unwrap();
    let cas = cas_dir().unwrap();
    assert!(
        entry.starts_with(&cas),
        "entry {:?} should start with cas_dir {:?}",
        entry,
        cas
    );
}

#[test]
fn cas_entry_short_hex_errs() {
    for s in ["", "a", "ab", "abc"] {
        let r = cas_entry(s);
        assert!(r.is_err(), "expected Err for {:?}, got {:?}", s, r);
    }
}

#[test]
fn cas_entry_minimal_4_char_hex_works() {
    let entry = cas_entry("abcd").expect("4-char hex should be Ok");
    assert_eq!(entry.parent().unwrap().file_name().unwrap(), "ab");
    assert_eq!(entry.file_name().unwrap(), "cd");
}
