//! Build-time guard tests for the v1.2 `pubgrub` dependency declaration.
//!
//! These tests read `Cargo.toml` at compile time via `include_str!` and
//! assert that the pubgrub dep is pinned to the 0.2.x line. The intent is
//! that nobody silently bumps it to 0.3 (which has an incompatible API)
//! without intentional review.

const CARGO_TOML: &str = include_str!("../Cargo.toml");

/// Normalize whitespace around `=` so `pubgrub = "0.2"` matches
/// `pubgrub  =  "0.2"` etc. We collapse runs of spaces/tabs to a single
/// space so simple `contains` checks work across formatting variants.
fn normalized() -> String {
    let mut out = String::with_capacity(CARGO_TOML.len());
    let mut prev_space = false;
    for ch in CARGO_TOML.chars() {
        if ch == ' ' || ch == '\t' {
            if !prev_space {
                out.push(' ');
            }
            prev_space = true;
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    out
}

#[test]
fn cargo_toml_declares_pubgrub_zero_two() {
    let n = normalized();
    let needle_a = r#"pubgrub = "0.2""#;
    let needle_b = r#"pubgrub = "0.2.1""#;
    let needle_c = r#"pubgrub = "0.2.0""#;
    assert!(
        n.contains(needle_a) || n.contains(needle_b) || n.contains(needle_c),
        "Cargo.toml must declare pubgrub at 0.2.x; got:\n{}",
        CARGO_TOML
    );
}

#[test]
fn cargo_toml_crate_version_is_one_two() {
    let n = normalized();
    // Accept 1.2.0 or any 1.2.x patch release.
    let has_exact = n.contains(r#"version = "1.2.0""#);
    let has_patch = (0..=99).any(|p| n.contains(&format!(r#"version = "1.2.{p}""#)));
    assert!(
        has_exact || has_patch,
        "Cargo.toml [package] version must be 1.2.x; got:\n{}",
        CARGO_TOML
    );
}

#[test]
fn cargo_toml_keeps_v1_deps() {
    let n = normalized();
    let required = [
        "clap",
        "tokio",
        "reqwest",
        "serde",
        "serde_json",
        "node-semver",
        "pubgrub",
    ];
    for name in required {
        // Match `name = ...` with normalized single-space separator. This
        // avoids accidental hits inside string values or comments.
        let needle = format!("{name} = ");
        assert!(
            n.contains(&needle),
            "Cargo.toml must keep v1 dep `{name}` (looking for `{needle}`); got:\n{}",
            CARGO_TOML
        );
    }
}

#[test]
fn cargo_toml_keeps_v1_metadata() {
    let n = normalized();
    assert!(
        n.contains(r#"repository = "https://github.com/nktkt/guroku""#),
        "Cargo.toml must declare repository = \"https://github.com/nktkt/guroku\"; got:\n{}",
        CARGO_TOML
    );
    assert!(
        n.contains(r#"license = "MIT""#),
        "Cargo.toml must declare license = \"MIT\"; got:\n{}",
        CARGO_TOML
    );
}

#[test]
fn cargo_toml_doesnt_advance_to_pubgrub_zero_three() {
    let n = normalized();
    // Any 0.3.x string form is forbidden until v1.3 intentionally bumps.
    let forbidden = [
        r#"pubgrub = "0.3""#,
        r#"pubgrub = "0.3.0""#,
        r#"pubgrub = "0.3.1""#,
        r#"pubgrub = "=0.3""#,
        r#"pubgrub = "^0.3""#,
        r#"pubgrub = "~0.3""#,
    ];
    for f in forbidden {
        assert!(
            !n.contains(f),
            "Cargo.toml must NOT bump pubgrub to 0.3 yet (found `{f}`). \
             If this is intentional for v1.3+, update this test to confirm \
             the bump was reviewed."
        );
    }
}
