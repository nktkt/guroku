use guroku::GurokuError;

#[test]
fn file_dep_missing_manifest_display() {
    let e = GurokuError::FileDepMissingManifest {
        path: "./pkg".into(),
    };
    let s = format!("{}", e);
    assert!(s.contains("./pkg"), "missing path: {}", s);
    assert!(s.contains("package.json"), "missing package.json: {}", s);
}

#[test]
fn git_command_failed_display() {
    let e = GurokuError::GitCommandFailed {
        url: "https://x/r.git".into(),
        detail: "fatal: ...".into(),
    };
    let s = format!("{}", e);
    assert!(s.contains("https://x/r.git"), "missing url: {}", s);
    assert!(s.contains("fatal"), "missing fatal: {}", s);
}

#[test]
fn audit_failed_display() {
    let e = GurokuError::AuditFailed("HTTP 503".into());
    let s = format!("{}", e);
    assert!(s.contains("audit"), "missing audit: {}", s);
    assert!(s.contains("503"), "missing 503: {}", s);
}

#[test]
fn invalid_override_display() {
    let e = GurokuError::InvalidOverride {
        name: "foo".into(),
        detail: "bad".into(),
    };
    let s = format!("{}", e);
    assert!(s.contains("foo"), "missing name: {}", s);
    assert!(s.contains("bad"), "missing detail: {}", s);
    assert!(s.contains("invalid override"), "missing phrase: {}", s);
}
