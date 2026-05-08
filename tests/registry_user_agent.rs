use guroku::registry::RegistryClient;

#[test]
fn crate_version_is_one_dot_zero() {
    assert!(env!("CARGO_PKG_VERSION").starts_with("1."));
}

#[test]
fn client_constructible_from_default_registry() {
    RegistryClient::with_default_registry().unwrap();
}

#[test]
fn client_constructible_from_custom_url() {
    RegistryClient::new(url::Url::parse("https://example.com/").unwrap()).unwrap();
}

#[test]
fn client_clonable() {
    let c = RegistryClient::with_default_registry().unwrap();
    let _c2 = c.clone();
}

#[test]
fn client_debug_prints_something() {
    let c = RegistryClient::with_default_registry().unwrap();
    assert!(!format!("{:?}", c).is_empty());
}

#[test]
fn user_agent_value_matches_format() {
    let ua = concat!("guroku/", env!("CARGO_PKG_VERSION"));
    assert!(ua.contains("guroku/1."));
}
