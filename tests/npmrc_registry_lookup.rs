use std::collections::BTreeMap;

use guroku::npmrc::Npmrc;
use guroku::registry::DEFAULT_REGISTRY;

fn npmrc(pairs: &[(&str, &str)]) -> Npmrc {
    let mut entries = BTreeMap::new();
    for (k, v) in pairs {
        entries.insert((*k).to_string(), (*v).to_string());
    }
    Npmrc { entries }
}

#[test]
fn registry_returns_default_when_not_set() {
    let n = npmrc(&[]);
    assert_eq!(n.registry(), DEFAULT_REGISTRY);
}

#[test]
fn registry_returns_configured_value() {
    let n = npmrc(&[("registry", "https://corp/")]);
    assert_eq!(n.registry(), "https://corp/");
}

#[test]
fn scoped_registry_lookup_with_at_prefix() {
    let n = npmrc(&[("@types:registry", "https://corp")]);
    assert_eq!(n.scoped_registry("@types"), Some("https://corp"));
}

#[test]
fn scoped_registry_lookup_without_at_prefix() {
    let n = npmrc(&[("@types:registry", "https://corp")]);
    assert_eq!(n.scoped_registry("types"), Some("https://corp"));
}

#[test]
fn scoped_registry_unknown_scope_is_none() {
    let n = npmrc(&[]);
    assert_eq!(n.scoped_registry("@unknown"), None);
}

#[test]
fn auth_token_for_host_returns_value() {
    let n = npmrc(&[("//registry.example.com/:_authToken", "secret")]);
    assert_eq!(n.auth_token("registry.example.com"), Some("secret"));
}

#[test]
fn auth_token_unknown_host_is_none() {
    let n = npmrc(&[]);
    assert_eq!(n.auth_token("unknown.host"), None);
}

#[test]
fn auth_token_strips_trailing_slash() {
    let n = npmrc(&[("//registry.example.com/:_authToken", "secret")]);
    assert_eq!(n.auth_token("registry.example.com/"), Some("secret"));
}
