use std::path::Path;

use guroku::cache::{home, metadata_cache_dir, metadata_cache_entry, metadata_etag_entry};

#[test]
fn metadata_cache_dir_under_home() {
    let h = home().unwrap();
    let m = metadata_cache_dir().unwrap();
    assert!(
        m.starts_with(&h),
        "metadata_cache_dir {:?} should start with home {:?}",
        m,
        h
    );
}

#[test]
fn metadata_cache_dir_ends_with_metadata() {
    let m = metadata_cache_dir().unwrap();
    assert_eq!(
        m.file_name().and_then(|s| s.to_str()),
        Some("metadata"),
        "last component should be 'metadata': {:?}",
        m
    );
    let parent = m.parent().expect("metadata dir has a parent");
    assert_eq!(
        parent.file_name().and_then(|s| s.to_str()),
        Some("cache"),
        "parent component should be 'cache': {:?}",
        parent
    );
}

#[test]
fn metadata_cache_entry_unscoped() {
    let p = metadata_cache_entry("lodash").unwrap();
    let s = p.to_string_lossy();
    assert!(
        s.contains("metadata") && s.ends_with("lodash.json"),
        "expected path under metadata/ ending with lodash.json: {:?}",
        p
    );
    assert!(p.ends_with(Path::new("lodash.json")), "{:?}", p);
}

#[test]
fn metadata_cache_entry_scoped() {
    let p = metadata_cache_entry("@types/node").unwrap();
    let s = p.to_string_lossy();
    assert!(
        s.contains("metadata") && s.ends_with("@types+node.json"),
        "expected scoped name to be sanitized to @types+node.json: {:?}",
        p
    );
}

#[test]
fn metadata_etag_entry_uses_etag_extension() {
    let p = metadata_etag_entry("lodash").unwrap();
    let s = p.to_string_lossy();
    assert!(
        s.contains("metadata") && s.ends_with("lodash.etag"),
        "expected path under metadata/ ending with lodash.etag: {:?}",
        p
    );
}

#[test]
fn etag_and_json_share_basename() {
    let json = metadata_cache_entry("@types/node").unwrap();
    let etag = metadata_etag_entry("@types/node").unwrap();
    assert_eq!(json.parent(), etag.parent(), "should live in same dir");
    assert_eq!(
        json.file_stem(),
        etag.file_stem(),
        "json and etag entries should share basename"
    );
    assert_eq!(json.extension().and_then(|s| s.to_str()), Some("json"));
    assert_eq!(etag.extension().and_then(|s| s.to_str()), Some("etag"));
}
