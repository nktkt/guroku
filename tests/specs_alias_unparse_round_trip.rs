use guroku::specs::{classify, unparse, DepSpec, GitRef};

fn alias(real_name: &str, range: &str) -> DepSpec {
    DepSpec::Alias {
        real_name: real_name.into(),
        inner: Box::new(DepSpec::Range(range.into())),
    }
}

#[test]
fn unparse_alias_unscoped() {
    assert_eq!(unparse(&alias("lodash", "^4")), "npm:lodash@^4");
}

#[test]
fn unparse_alias_scoped() {
    assert_eq!(unparse(&alias("@types/node", "^20")), "npm:@types/node@^20");
}

#[test]
fn round_trip_alias_unscoped() {
    let spec = alias("lodash", "^4");
    assert_eq!(classify(&unparse(&spec)), spec);
}

#[test]
fn round_trip_alias_scoped() {
    let spec = alias("@types/node", "^20");
    assert_eq!(classify(&unparse(&spec)), spec);
}

#[test]
fn round_trip_range_unaffected() {
    let spec = DepSpec::Range("^1.2.3".into());
    assert_eq!(classify(&unparse(&spec)), spec);
}

#[test]
fn round_trip_file_unaffected() {
    let spec = DepSpec::File("./pkg".into());
    assert_eq!(classify(&unparse(&spec)), spec);
}

#[test]
fn round_trip_git_unaffected() {
    let spec = DepSpec::Git(GitRef {
        url: "https://x/r.git".into(),
        revision: Some("v1".into()),
    });
    assert_eq!(classify(&unparse(&spec)), spec);
}
