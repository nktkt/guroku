use guroku::cache::{cas_dir, git_cache_dir, home, metadata_cache_dir};

#[test]
fn git_cache_under_home() {
    let h = home().unwrap();
    let g = git_cache_dir().unwrap();
    assert!(
        g.starts_with(&h),
        "git_cache_dir {:?} should start with home {:?}",
        g,
        h
    );
}

#[test]
fn git_cache_ends_with_git() {
    let g = git_cache_dir().unwrap();
    assert_eq!(
        g.file_name().and_then(|s| s.to_str()),
        Some("git"),
        "last component should be 'git': {:?}",
        g
    );
    let parent = g.parent().expect("git cache dir has a parent");
    assert_eq!(
        parent.file_name().and_then(|s| s.to_str()),
        Some("cache"),
        "parent component should be 'cache': {:?}",
        parent
    );
}

#[test]
fn git_cache_distinct_from_metadata() {
    let g = git_cache_dir().unwrap();
    let m = metadata_cache_dir().unwrap();
    assert_ne!(g, m, "git_cache_dir must differ from metadata_cache_dir");
}

#[test]
fn git_cache_distinct_from_cas() {
    let g = git_cache_dir().unwrap();
    let c = cas_dir().unwrap();
    assert_ne!(g, c, "git_cache_dir must differ from cas_dir");
}
