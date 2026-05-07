use guroku::cache;

#[test]
fn store_dir_is_under_home() {
    let home = cache::home().unwrap();
    let store = cache::store_dir().unwrap();
    assert!(
        store.starts_with(&home),
        "store_dir {:?} should start with home {:?}",
        store,
        home
    );
}

#[test]
fn store_dir_ends_with_store() {
    let store = cache::store_dir().unwrap();
    assert!(
        store.ends_with("store"),
        "store_dir {:?} should end with 'store'",
        store
    );
}

#[test]
fn package_dir_unscoped() {
    let p = cache::package_dir("lodash", "4.17.21").unwrap();
    let s = p.to_string_lossy();
    assert!(
        s.ends_with("store/lodash/4.17.21"),
        "package_dir was {:?}",
        s
    );
}

#[test]
fn package_dir_scoped() {
    let p = cache::package_dir("@types/node", "20.0.0").unwrap();
    let s = p.to_string_lossy();
    assert!(
        s.ends_with("store/@types+node/20.0.0"),
        "package_dir was {:?}",
        s
    );
}

#[test]
fn tarball_cache_dir_path() {
    let p = cache::tarball_cache_dir().unwrap();
    let s = p.to_string_lossy();
    assert!(
        s.ends_with("cache/tarballs"),
        "tarball_cache_dir was {:?}",
        s
    );
}
