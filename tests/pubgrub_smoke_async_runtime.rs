//! Smoke tests confirming the async runtime entry point works for the
//! resolver layer. Uses `#[tokio::test]` since `tokio` is a top-level
//! dependency with the `full` feature set.
//!
//! These tests pass empty `roots`, so no network I/O happens — the
//! `RegistryClient` is constructed only to satisfy the function signature.

#[tokio::test]
async fn pubgrub_with_zero_roots_returns_empty() {
    let client = guroku::registry::RegistryClient::with_default_registry()
        .expect("default registry client should construct");
    let manifest = guroku::manifest::Manifest::default();
    let roots: Vec<(String, String)> = vec![];
    let resolution = guroku::pubgrub_resolver::resolve_with_pubgrub(&client, &roots, &manifest)
        .await
        .expect("empty roots should resolve trivially");
    assert!(
        resolution.is_empty(),
        "no roots should yield no packages, got {} entries",
        resolution.len()
    );
}

#[tokio::test]
async fn bfs_with_zero_roots_returns_empty() {
    let client = guroku::registry::RegistryClient::with_default_registry()
        .expect("default registry client should construct");
    let manifest = guroku::manifest::Manifest::default();
    let roots: Vec<(String, String)> = vec![];
    let resolution = guroku::resolver::resolve_with_manifest_overrides(&client, &roots, &manifest)
        .await
        .expect("empty roots should resolve trivially via BFS");
    assert!(
        resolution.is_empty(),
        "BFS with no roots should yield no packages, got {} entries",
        resolution.len()
    );
}

#[tokio::test]
async fn pubgrub_synthetic_root_does_not_appear_in_resolution() {
    let client = guroku::registry::RegistryClient::with_default_registry()
        .expect("default registry client should construct");
    let manifest = guroku::manifest::Manifest::default();
    let roots: Vec<(String, String)> = vec![];
    let resolution = guroku::pubgrub_resolver::resolve_with_pubgrub(&client, &roots, &manifest)
        .await
        .expect("empty roots resolve");
    assert!(
        !resolution.packages.contains_key("$guroku-root"),
        "synthetic root sentinel must never leak into the final Resolution"
    );
}
