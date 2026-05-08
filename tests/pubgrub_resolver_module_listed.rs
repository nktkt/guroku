const LIB_RS: &str = include_str!("../src/lib.rs");

#[test]
fn pubgrub_resolver_module_declared() {
    assert!(
        LIB_RS.contains("pub mod pubgrub_resolver;"),
        "expected `pub mod pubgrub_resolver;` in src/lib.rs"
    );
}

#[test]
fn pub_mod_resolver_still_present() {
    assert!(
        LIB_RS.contains("pub mod resolver;"),
        "v1.0/v1.1 resolver module must remain"
    );
}

#[test]
fn pub_mod_overrides_still_present() {
    assert!(LIB_RS.contains("pub mod overrides;"));
}

#[test]
fn pub_mod_specs_still_present() {
    assert!(LIB_RS.contains("pub mod specs;"));
}

#[test]
fn pub_mod_prelude_still_present() {
    assert!(LIB_RS.contains("pub mod prelude;"));
}
