use guroku::registry::RegistryClient;

#[test]
fn without_http_cache_returns_self() {
    let _client = RegistryClient::with_default_registry()
        .unwrap()
        .without_http_cache();
}

#[test]
fn client_constructible_from_custom_base() {
    let base = url::Url::parse("https://example.com/").unwrap();
    let result = RegistryClient::new(base);
    assert!(result.is_ok());
}

#[test]
fn with_default_registry_smoke() {
    let c = RegistryClient::with_default_registry().unwrap();
    let _ = format!("{:?}", c.clone());
}

#[test]
fn without_http_cache_is_chainable() {
    let base = url::Url::parse("https://example.com/").unwrap();
    let _client = RegistryClient::new(base).unwrap().without_http_cache();
}
