use guroku::prelude::*;
use std::path::Path;

#[test]
fn embedding_smoke_compiles() {
    // We don't have a real package.json on the test path; this test is
    // about API shape, not behaviour. Each line below is what an
    // embedder would write; we just exercise the type-level wiring.
    fn _hypothetical() -> Result<()> {
        let _manifest: Manifest = Manifest::default();
        let _client: RegistryClient = RegistryClient::with_default_registry()?;
        let _roots: Vec<(String, String)> = vec![("ms".into(), "^2".into())];
        // resolver::resolve is async; we sanity-check the type wiring with a
        // synchronous Resolution::default() instead of building a future.
        let _: Resolution = Resolution::default();
        Ok(())
    }
    let _ = _hypothetical();
}

#[test]
fn manifest_read_signature_compiles() {
    fn _check(p: &Path) -> Result<Manifest> {
        Manifest::read_from(p)
    }
    let _ = _check;
}

#[test]
fn lockfile_read_signature_compiles() {
    fn _check(p: &Path) -> Result<Lockfile> {
        Lockfile::read_from(p)
    }
    let _ = _check;
}

#[test]
fn classify_spec_returns_dep_spec() {
    let s: DepSpec = classify_spec("^1.2.3");
    assert!(matches!(s, DepSpec::Range(_)));
}
