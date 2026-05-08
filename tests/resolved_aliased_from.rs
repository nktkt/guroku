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
fn non_alias_has_none() {
    let r = make_resolved("lodash", "4.17.21", None);
    assert!(r.aliased_from.is_none());
}

#[test]
fn alias_carries_real_name() {
    let r = make_resolved("my-lodash", "4.17.21", Some("lodash".into()));
    assert_eq!(r.aliased_from.as_deref(), Some("lodash"));
}

#[test]
fn aliased_from_distinct_from_info_name() {
    let r = make_resolved("my-lodash", "4.17.21", Some("lodash".into()));
    assert_eq!(r.info.name, "my-lodash");
    assert_eq!(r.aliased_from.as_deref(), Some("lodash"));
    assert_ne!(r.info.name.as_str(), r.aliased_from.as_deref().unwrap());
}

#[test]
fn resolution_iter_yields_aliased_from() {
    let mut packages: BTreeMap<String, guroku::resolver::Resolved> = BTreeMap::new();
    packages.insert(
        "my-lodash".to_string(),
        make_resolved("my-lodash", "4.17.21", Some("lodash".into())),
    );
    let resolution = guroku::resolver::Resolution { packages };

    let mut count = 0usize;
    for (key, resolved) in resolution.packages.iter() {
        assert_eq!(key, "my-lodash");
        assert_eq!(resolved.aliased_from.as_deref(), Some("lodash"));
        count += 1;
    }
    assert_eq!(count, 1);
}
