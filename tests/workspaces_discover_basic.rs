use std::fs;
use std::path::Path;

use guroku::workspaces::discover;
use tempfile::TempDir;

fn write(p: &Path, body: &str) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, body).unwrap();
}

#[test]
fn discovers_two_workspace_packages() {
    let td = TempDir::new().unwrap();
    let root = td.path();
    write(
        &root.join("package.json"),
        r#"{"name":"root","version":"0.1.0","workspaces":["packages/*"]}"#,
    );
    write(
        &root.join("packages/a/package.json"),
        r#"{"name":"@acme/a","version":"0.1.0"}"#,
    );
    write(
        &root.join("packages/b/package.json"),
        r#"{"name":"@acme/b","version":"0.1.0"}"#,
    );

    let ws = discover(root).expect("discover ok");
    assert_eq!(ws.len(), 2, "expected 2 workspaces, got {}", ws.len());

    let names: Vec<&str> = ws.iter().filter_map(|w| w.name()).collect();
    assert!(names.contains(&"@acme/a"), "missing @acme/a in {:?}", names);
    assert!(names.contains(&"@acme/b"), "missing @acme/b in {:?}", names);

    let roots: Vec<_> = ws.iter().map(|w| w.root.clone()).collect();
    assert!(
        roots.iter().any(|r| r.ends_with("packages/a")),
        "no root ending packages/a in {:?}",
        roots
    );
    assert!(
        roots.iter().any(|r| r.ends_with("packages/b")),
        "no root ending packages/b in {:?}",
        roots
    );
}

#[test]
fn no_workspaces_field_returns_empty() {
    let td = TempDir::new().unwrap();
    write(
        &td.path().join("package.json"),
        r#"{"name":"root","version":"0.1.0"}"#,
    );
    let ws = discover(td.path()).expect("discover ok");
    assert_eq!(ws.len(), 0);
}

#[test]
fn missing_root_package_json_returns_empty() {
    let td = TempDir::new().unwrap();
    let ws = discover(td.path()).expect("discover ok");
    assert_eq!(ws.len(), 0);
}

#[test]
fn subdir_without_package_json_skipped() {
    let td = TempDir::new().unwrap();
    let root = td.path();
    write(
        &root.join("package.json"),
        r#"{"name":"root","version":"0.1.0","workspaces":["packages/*"]}"#,
    );
    write(
        &root.join("packages/a/package.json"),
        r#"{"name":"@acme/a","version":"0.1.0"}"#,
    );
    fs::create_dir_all(root.join("packages/empty")).unwrap();

    let ws = discover(root).expect("discover ok");
    assert_eq!(ws.len(), 1);
    assert_eq!(ws[0].name(), Some("@acme/a"));
}

#[test]
fn globs_can_be_specific_paths() {
    let td = TempDir::new().unwrap();
    let root = td.path();
    write(
        &root.join("package.json"),
        r#"{"name":"root","version":"0.1.0","workspaces":["packages/a"]}"#,
    );
    write(
        &root.join("packages/a/package.json"),
        r#"{"name":"@acme/a","version":"0.1.0"}"#,
    );
    write(
        &root.join("packages/b/package.json"),
        r#"{"name":"@acme/b","version":"0.1.0"}"#,
    );

    let ws = discover(root).expect("discover ok");
    assert_eq!(ws.len(), 1);
    assert_eq!(ws[0].name(), Some("@acme/a"));
}

#[test]
fn result_sorted_alphabetically() {
    let td = TempDir::new().unwrap();
    let root = td.path();
    write(
        &root.join("package.json"),
        r#"{"name":"root","version":"0.1.0","workspaces":["packages/*"]}"#,
    );
    for n in ["c", "a", "b"] {
        write(
            &root.join(format!("packages/{n}/package.json")),
            &format!(r#"{{"name":"@acme/{n}","version":"0.1.0"}}"#),
        );
    }

    let ws = discover(root).expect("discover ok");
    let roots: Vec<_> = ws.iter().map(|w| w.root.clone()).collect();
    let mut sorted = roots.clone();
    sorted.sort();
    assert_eq!(roots, sorted, "expected sorted-by-path order: {:?}", roots);
    assert_eq!(roots.len(), 3);
}
