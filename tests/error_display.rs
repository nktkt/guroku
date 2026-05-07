use guroku::error::GurokuError;

#[test]
fn package_not_found_display() {
    let e = GurokuError::PackageNotFound {
        name: "lodash".into(),
    };
    let s = format!("{}", e);
    assert!(s.contains("lodash"), "missing name: {}", s);
    assert!(s.contains("not found"), "missing 'not found': {}", s);
}

#[test]
fn no_matching_version_display() {
    let e = GurokuError::NoMatchingVersion {
        name: "lodash".into(),
        spec: "^99".into(),
    };
    let s = format!("{}", e);
    assert!(s.contains("lodash"), "missing name: {}", s);
    assert!(s.contains("^99"), "missing spec: {}", s);
}

#[test]
fn unsupported_integrity_display() {
    let e = GurokuError::UnsupportedIntegrity("sha1".into());
    let s = format!("{}", e);
    assert!(s.contains("sha1"), "missing algo: {}", s);
    assert!(s.contains("unsupported"), "missing 'unsupported': {}", s);
}

#[test]
fn tarball_display() {
    let e = GurokuError::Tarball("bad header".into());
    let s = format!("{}", e);
    assert!(s.contains("bad header"), "missing detail: {}", s);
}

#[test]
fn no_cache_dir_display() {
    let e = GurokuError::NoCacheDir;
    let s = format!("{}", e);
    assert!(
        s.contains("cache directory"),
        "missing 'cache directory': {}",
        s
    );
}
