use std::fs;
use tempfile::TempDir;

use guroku::linker::link_flat;

#[test]
fn links_files_into_node_modules() {
    let store = TempDir::new().unwrap();
    let nm = TempDir::new().unwrap();

    let store_foo = store.path().join("foo");
    fs::create_dir_all(&store_foo).unwrap();
    fs::write(store_foo.join("package.json"), r#"{"name":"foo"}"#).unwrap();
    fs::write(store_foo.join("index.js"), "module.exports = 1;").unwrap();

    link_flat(&store_foo, nm.path(), "foo").unwrap();

    let pkg = nm.path().join("foo").join("package.json");
    let idx = nm.path().join("foo").join("index.js");
    assert!(pkg.exists());
    assert!(idx.exists());
    assert_eq!(fs::read_to_string(&pkg).unwrap(), r#"{"name":"foo"}"#);
    assert_eq!(fs::read_to_string(&idx).unwrap(), "module.exports = 1;");
}

#[test]
fn links_nested_directories() {
    let store = TempDir::new().unwrap();
    let nm = TempDir::new().unwrap();

    let store_foo = store.path().join("foo");
    let lib = store_foo.join("lib");
    fs::create_dir_all(&lib).unwrap();
    fs::write(lib.join("util.js"), "exports.u = 1;").unwrap();

    link_flat(&store_foo, nm.path(), "foo").unwrap();

    let nested = nm.path().join("foo").join("lib").join("util.js");
    assert!(nested.exists());
    assert_eq!(fs::read_to_string(&nested).unwrap(), "exports.u = 1;");
}

#[test]
fn overwrites_existing_link() {
    let store = TempDir::new().unwrap();
    let nm = TempDir::new().unwrap();

    let existing = nm.path().join("foo");
    fs::create_dir_all(&existing).unwrap();
    fs::write(existing.join("old.txt"), "old").unwrap();

    let store_foo = store.path().join("foo");
    fs::create_dir_all(&store_foo).unwrap();
    fs::write(store_foo.join("new.txt"), "new").unwrap();

    link_flat(&store_foo, nm.path(), "foo").unwrap();

    assert!(!nm.path().join("foo").join("old.txt").exists());
    let new_file = nm.path().join("foo").join("new.txt");
    assert!(new_file.exists());
    assert_eq!(fs::read_to_string(&new_file).unwrap(), "new");
}

#[test]
fn creates_node_modules_if_missing() {
    let store = TempDir::new().unwrap();
    let parent = TempDir::new().unwrap();

    let store_foo = store.path().join("foo");
    fs::create_dir_all(&store_foo).unwrap();
    fs::write(store_foo.join("index.js"), "x").unwrap();

    let nm = parent
        .path()
        .join("does_not_exist_yet")
        .join("node_modules");
    assert!(!nm.exists());

    link_flat(&store_foo, &nm, "foo").unwrap();

    assert!(nm.exists());
    assert!(nm.join("foo").join("index.js").exists());
}
