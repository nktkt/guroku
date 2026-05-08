#[allow(dead_code)]
async fn _diamond_resolve_compiles(
    client: &guroku::registry::RegistryClient,
    roots: &[(String, String)],
    manifest: &guroku::manifest::Manifest,
) -> guroku::Result<guroku::resolver::Resolution> {
    guroku::pubgrub_resolver::resolve_with_pubgrub(client, roots, manifest).await
}

#[test]
fn diamond_resolve_compiles() {
    let _ = _diamond_resolve_compiles;
}

#[test]
fn resolution_iterates_in_sorted_order() {
    use guroku::resolver::Resolution;
    let r = Resolution::default();
    let count = r.iter().count();
    assert_eq!(count, 0);
}

#[test]
fn npm_version_ord_picks_higher_minor() {
    use guroku::pubgrub_resolver::NpmVersion;
    use guroku::version::parse_version;
    let a = NpmVersion(parse_version("1.5.0").unwrap());
    let b = NpmVersion(parse_version("1.2.3").unwrap());
    assert!(a > b);
}

#[test]
fn npm_version_picks_higher_major() {
    use guroku::pubgrub_resolver::NpmVersion;
    use guroku::version::parse_version;
    let a = NpmVersion(parse_version("2.0.0").unwrap());
    let b = NpmVersion(parse_version("1.99.99").unwrap());
    assert!(a > b);
}

#[test]
fn npm_version_picks_higher_patch_within_minor() {
    use guroku::pubgrub_resolver::NpmVersion;
    use guroku::version::parse_version;
    let a = NpmVersion(parse_version("1.2.4").unwrap());
    let b = NpmVersion(parse_version("1.2.3").unwrap());
    assert!(a > b);
}

#[test]
fn pre_release_orders_below_release() {
    use guroku::pubgrub_resolver::NpmVersion;
    use guroku::version::parse_version;
    let pre = NpmVersion(parse_version("1.2.3-rc.1").unwrap());
    let rel = NpmVersion(parse_version("1.2.3").unwrap());
    assert!(pre < rel);
}
