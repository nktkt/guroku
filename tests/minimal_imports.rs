//! Ensures embedders can drive guroku with the smallest possible import block.
//!
//! Two supported styles:
//!   1. `use guroku::prelude::*;` — pulls in everything an embedder typically needs.
//!   2. `use guroku::{Result, GurokuError};` — explicit per-call qualified paths.

#[test]
fn prelude_only_imports_compile() {
    use guroku::prelude::*;

    let _ = Manifest::default();
    let _ = Lockfile::new();
    let _ = parse_range("^1").unwrap();
    let _ = parse_version("1.0.0").unwrap();
    let _ = max_satisfying(["1.0.0"].iter().copied(), &parse_range("^1").unwrap());
    let _ = classify_spec("file:./x");
    let _ = RegistryClient::with_default_registry().unwrap();
    let _: Resolution = Resolution::default();
}

#[test]
fn crate_root_only_imports_compile() {
    use guroku::{GurokuError, Result};

    let _ = guroku::manifest::Manifest::default();
    let _ = guroku::lockfile::Lockfile::new();
    let range = guroku::version::parse_range("^1").unwrap();
    let _ = guroku::version::parse_version("1.0.0").unwrap();
    let _ = guroku::version::max_satisfying(["1.0.0"].iter().copied(), &range);
    let _ = guroku::specs::classify("file:./x");
    let _ = guroku::registry::RegistryClient::with_default_registry().unwrap();
    let _: guroku::resolver::Resolution = guroku::resolver::Resolution::default();

    // Confirm the crate-root error/result aliases are usable here too.
    let _: Result<()> = Ok(());
    let _: GurokuError = GurokuError::Other("x".into());
}

#[test]
fn error_result_at_crate_root() {
    fn _ok() -> guroku::Result<i32> {
        Ok(42)
    }
    assert_eq!(_ok().unwrap(), 42);
    let _: guroku::GurokuError = guroku::GurokuError::Other("x".into());
}
