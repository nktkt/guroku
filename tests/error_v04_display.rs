use guroku::error::GurokuError;

#[test]
fn script_failed_display() {
    let err = GurokuError::ScriptFailed {
        script: "build".into(),
        status: 2,
    };
    let s = format!("{}", err);
    assert!(s.contains("build"), "missing script name: {}", s);
    assert!(s.contains("status 2"), "missing status: {}", s);
}

#[test]
fn script_spawn_failed_display() {
    let err = GurokuError::ScriptSpawnFailed {
        script: "postinstall".into(),
        detail: "No such file or directory".into(),
    };
    let s = format!("{}", err);
    assert!(s.contains("postinstall"), "missing script name: {}", s);
    assert!(s.contains("No such file"), "missing detail: {}", s);
}

#[test]
fn no_such_script_display() {
    let err = GurokuError::NoSuchScript {
        name: "lint".into(),
    };
    let s = format!("{}", err);
    assert!(s.contains("lint"), "missing name: {}", s);
    assert!(s.to_lowercase().contains("no"), "missing 'no': {}", s);
}

#[test]
fn workspace_misconfigured_display() {
    let err = GurokuError::WorkspaceMisconfigured("invalid glob".into());
    let s = format!("{}", err);
    assert!(
        s.contains("workspaces misconfigured"),
        "missing prefix: {}",
        s
    );
    assert!(s.contains("invalid glob"), "missing detail: {}", s);
}

#[test]
fn bin_not_found_display() {
    let err = GurokuError::BinNotFound {
        name: "prettier".into(),
    };
    let s = format!("{}", err);
    assert!(s.contains("prettier"), "missing bin name: {}", s);
    assert!(s.contains("node_modules/.bin"), "missing path hint: {}", s);
}
