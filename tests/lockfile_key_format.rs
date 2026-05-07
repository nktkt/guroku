use guroku::lockfile::{Lockfile, PackageLock};

fn entry() -> PackageLock {
    PackageLock {
        resolved: "https://example.com/x.tgz".into(),
        integrity: None,
        dependencies: Default::default(),
    }
}

#[test]
fn key_unscoped() {
    assert_eq!(Lockfile::key("lodash", "4.17.21"), "lodash@4.17.21");
}

#[test]
fn key_scoped() {
    assert_eq!(
        Lockfile::key("@types/node", "20.10.0"),
        "@types/node@20.10.0"
    );
}

#[test]
fn contains_returns_true_after_insert() {
    let mut lock = Lockfile::new();
    lock.insert("lodash", "4.17.21", entry());
    assert!(lock.contains("lodash", "4.17.21"));
}

#[test]
fn contains_returns_false_for_other_version() {
    let mut lock = Lockfile::new();
    lock.insert("lodash", "4.17.21", entry());
    assert!(!lock.contains("lodash", "4.17.20"));
}

#[test]
fn contains_distinguishes_scoped_from_unscoped() {
    let mut lock = Lockfile::new();
    lock.insert("@types/node", "1.0.0", entry());
    assert!(!lock.contains("types/node", "1.0.0"));
    assert!(lock.contains("@types/node", "1.0.0"));
}

#[test]
fn key_round_trips_through_packages_map() {
    let mut lock = Lockfile::new();
    let name = "lodash";
    let version = "4.17.21";
    lock.insert(name, version, entry());
    let keys: Vec<&String> = lock.packages.keys().collect();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0], &Lockfile::key(name, version));
}
