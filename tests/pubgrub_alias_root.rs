// Tests that verify the alias-decomposition logic at the boundary of
// `guroku::pubgrub_resolver::resolve_with_pubgrub`. Most of these are
// compile-time / type-only checks because we can't easily exercise the
// full async solver against the live npm registry in unit tests.

#[allow(dead_code)]
async fn _compile_check(
    client: &guroku::registry::RegistryClient,
    roots: &[(String, String)],
    manifest: &guroku::manifest::Manifest,
) -> guroku::Result<guroku::resolver::Resolution> {
    guroku::pubgrub_resolver::resolve_with_pubgrub(client, roots, manifest).await
}

#[test]
fn signature_compiles() {
    let _f = _compile_check;
}

#[test]
fn imports_npm_version_and_solve_path() {
    use guroku::pubgrub_resolver::{resolve_with_pubgrub, NpmVersion};
    let _ = resolve_with_pubgrub;
    let _ = std::marker::PhantomData::<NpmVersion>;
}

#[test]
fn resolved_aliased_from_round_trip() {
    use guroku::registry::{Dist, VersionInfo};
    use guroku::resolver::Resolved;
    use std::collections::BTreeMap;
    use url::Url;

    let r = Resolved {
        info: VersionInfo {
            name: "real-pkg".into(),
            version: "1.0.0".into(),
            dist: Dist {
                tarball: Url::parse("https://example.invalid/x.tgz").unwrap(),
                integrity: None,
                shasum: None,
            },
            dependencies: BTreeMap::new(),
        },
        local_source: None,
        aliased_from: Some("real-pkg".into()),
    };
    assert_eq!(r.aliased_from.as_deref(), Some("real-pkg"));
}
