//! v1.0 cache-path layout stability test.
//!
//! Failing here means a path moved (and any user with v0.x state has just
//! been orphaned). Treat changes to this file as part of a deliberate
//! migration, not a refactor.

use std::path::{Component, Path};

use guroku::cache;

fn last_segment(p: &Path) -> String {
    p.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn components_contain_in_order(p: &Path, needles: &[&str]) -> bool {
    let segs: Vec<String> = p
        .components()
        .filter_map(|c| match c {
            Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    let mut idx = 0usize;
    for seg in &segs {
        if idx < needles.len() && seg == needles[idx] {
            idx += 1;
        }
    }
    idx == needles.len()
}

#[test]
fn cache_paths_v1_layout_is_stable() {
    // 1. home_path_ends_with_dot_guroku
    let home = cache::home().expect("home()");
    assert_eq!(
        last_segment(&home),
        ".guroku",
        "home() must end with .guroku, got {:?}",
        home
    );

    // 2. cas_under_home
    let cas = cache::cas_dir().expect("cas_dir()");
    assert!(
        cas.starts_with(&home),
        "cas_dir() {:?} must start with home {:?}",
        cas,
        home
    );

    // 3. metadata_cache_under_home_cache
    let meta = cache::metadata_cache_dir().expect("metadata_cache_dir()");
    assert!(
        components_contain_in_order(&meta, &["cache", "metadata"]),
        "metadata_cache_dir() {:?} must contain cache/ then metadata/ in its ancestors",
        meta
    );

    // 4. git_cache_under_home_cache
    let git = cache::git_cache_dir().expect("git_cache_dir()");
    assert!(
        components_contain_in_order(&git, &["cache", "git"]),
        "git_cache_dir() {:?} must contain cache/ then git/ in its ancestors",
        git
    );

    // 5. store_dir_under_home_for_legacy_compat
    let store = cache::store_dir().expect("store_dir()");
    assert!(
        store.starts_with(&home),
        "store_dir() {:?} must start with home {:?}",
        store,
        home
    );
    assert!(
        store.ends_with("store"),
        "store_dir() {:?} must end with 'store' (legacy compat)",
        store
    );

    // 6. tarball_cache_dir_reserved
    let tarballs = cache::tarball_cache_dir().expect("tarball_cache_dir()");
    assert!(
        tarballs.ends_with("tarballs"),
        "tarball_cache_dir() {:?} must end with 'tarballs'",
        tarballs
    );

    // 7. metadata_entry_uses_safe_segment
    let scoped = cache::metadata_cache_entry("@types/node").expect("metadata_cache_entry");
    let scoped_s = scoped.to_string_lossy();
    assert!(
        scoped_s.contains("@types+node.json"),
        "metadata_cache_entry(\"@types/node\") {:?} must contain '@types+node.json'",
        scoped_s
    );
    // Sanity: etag entry should also resolve for the same input.
    let _etag = cache::metadata_etag_entry("@types/node").expect("metadata_etag_entry");

    // safe_segment is the primitive these entries lean on; pin its contract.
    assert_eq!(cache::safe_segment("@types/node"), "@types+node");

    // 8. cas_entry_two_char_prefix
    let hex: String = "abcdef0123456789".repeat(13).chars().take(128).collect();
    let cas_path = cache::cas_entry(&hex).expect("cas_entry");
    let has_ab_segment = cas_path.components().any(|c| {
        matches!(
            c,
            Component::Normal(s) if s.to_string_lossy() == "ab"
        )
    });
    assert!(
        has_ab_segment,
        "cas_entry({}) {:?} must include 'ab' as a single path segment",
        &hex[..8],
        cas_path
    );
    // And the remaining 126 hex chars should be the tail filename.
    assert_eq!(
        last_segment(&cas_path),
        hex[2..],
        "cas_entry tail must be the post-prefix hex"
    );

    // 9. package_dir_legacy_layout
    let pkg = cache::package_dir("lodash", "4.17.21").expect("package_dir");
    assert!(
        components_contain_in_order(&pkg, &["store", "lodash", "4.17.21"]),
        "package_dir(\"lodash\",\"4.17.21\") {:?} must end with store/lodash/4.17.21",
        pkg
    );
    // Tail must specifically be the version dir.
    assert_eq!(last_segment(&pkg), "4.17.21");
}
