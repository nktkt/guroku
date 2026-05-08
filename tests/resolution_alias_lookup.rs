use std::collections::BTreeMap;

fn make_resolved(
    name: &str,
    version: &str,
    aliased_from: Option<String>,
) -> guroku::resolver::Resolved {
    use guroku::registry::{Dist, VersionInfo};
    use url::Url;
    guroku::resolver::Resolved {
        info: VersionInfo {
            name: name.to_string(),
            version: version.to_string(),
            dist: Dist {
                tarball: Url::parse("https://example.com/x.tgz").unwrap(),
                integrity: None,
                shasum: None,
            },
            dependencies: BTreeMap::new(),
        },
        local_source: None,
        aliased_from,
    }
}

#[test]
fn lookup_by_local_name_succeeds() {
    let mut packages: BTreeMap<String, guroku::resolver::Resolved> = BTreeMap::new();
    packages.insert(
        "my-lodash".to_string(),
        make_resolved("lodash", "4.17.21", Some("lodash".into())),
    );
    let resolution = guroku::resolver::Resolution { packages };
    let got = resolution.packages.get("my-lodash");
    assert!(got.is_some());
    assert_eq!(got.unwrap().info.name, "lodash");
}

#[test]
fn lookup_by_real_name_returns_none() {
    let mut packages: BTreeMap<String, guroku::resolver::Resolved> = BTreeMap::new();
    packages.insert(
        "my-lodash".to_string(),
        make_resolved("lodash", "4.17.21", Some("lodash".into())),
    );
    let resolution = guroku::resolver::Resolution { packages };
    assert!(!resolution.packages.contains_key("lodash"));
}

#[test]
fn len_counts_local_names() {
    let mut packages: BTreeMap<String, guroku::resolver::Resolved> = BTreeMap::new();
    packages.insert(
        "alias-one".to_string(),
        make_resolved("lodash", "4.17.21", Some("lodash".into())),
    );
    packages.insert(
        "alias-two".to_string(),
        make_resolved("lodash", "4.17.21", Some("lodash".into())),
    );
    let resolution = guroku::resolver::Resolution { packages };
    assert_eq!(resolution.packages.len(), 2);
}

#[test]
fn iteration_order_is_alphabetical() {
    let mut packages: BTreeMap<String, guroku::resolver::Resolved> = BTreeMap::new();
    packages.insert(
        "z-lodash".to_string(),
        make_resolved("lodash", "4.17.21", Some("lodash".into())),
    );
    packages.insert(
        "a-lodash".to_string(),
        make_resolved("lodash", "4.17.21", Some("lodash".into())),
    );
    packages.insert(
        "m-lodash".to_string(),
        make_resolved("lodash", "4.17.21", Some("lodash".into())),
    );
    let resolution = guroku::resolver::Resolution { packages };
    let keys: Vec<&str> = resolution.packages.keys().map(|s| s.as_str()).collect();
    assert_eq!(keys, vec!["a-lodash", "m-lodash", "z-lodash"]);
}

#[test]
fn aliased_from_distinguishes_aliases_from_regulars() {
    let mut packages: BTreeMap<String, guroku::resolver::Resolved> = BTreeMap::new();
    packages.insert(
        "lodash".to_string(),
        make_resolved("lodash", "4.17.21", None),
    );
    packages.insert(
        "my-lodash".to_string(),
        make_resolved("lodash", "4.17.21", Some("lodash".into())),
    );
    packages.insert(
        "other-lodash".to_string(),
        make_resolved("lodash", "4.17.21", Some("lodash".into())),
    );
    let resolution = guroku::resolver::Resolution { packages };
    let aliased = resolution
        .packages
        .values()
        .filter(|r| r.aliased_from.is_some())
        .count();
    let regular = resolution
        .packages
        .values()
        .filter(|r| r.aliased_from.is_none())
        .count();
    assert_eq!(aliased, 2);
    assert_eq!(regular, 1);
}
