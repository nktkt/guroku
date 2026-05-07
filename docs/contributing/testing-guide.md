# Testing Guide

This guide is for contributors writing or modifying tests in guroku. It covers
how the test suite is laid out, the conventions we hold to, and the small set
of rules that keep the suite fast and reliable.

If you are adding behaviour, you are adding a test. If you are fixing a bug,
you are adding a test that would have caught it.

## Test layout

guroku follows the standard Cargo layout:

- `tests/<file>.rs` — integration tests. Each file is compiled as its own
  test binary. Files cannot share helpers via `mod` declarations across files;
  put shared helpers in `tests/common/mod.rs` and `mod common;` from each test
  file that needs them.
- `src/<module>.rs` — unit tests live inline at the bottom of the module they
  exercise, inside a `#[cfg(test)] mod tests { ... }` block. Use this for
  tightly-coupled unit tests where the test needs access to private items.

Rule of thumb: if the test only touches public API, put it under `tests/`. If
it touches private functions or constants, inline it in `src/`.

```rust
// src/manifest.rs
pub fn parse(input: &str) -> Result<Manifest, ManifestError> { /* ... */ }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_manifest() {
        let m = parse(r#"{"name":"x","version":"0.1.0"}"#).unwrap();
        assert_eq!(m.name, "x");
    }
}
```

## Naming conventions

One tested behaviour per file. The filename describes the behaviour, not the
module:

- `tests/lockfile_roundtrip.rs`
- `tests/registry_resolve_caret.rs`
- `tests/cli_install_creates_node_modules.rs`

Function names are snake_case sentences that read as assertions about the
system under test:

- `parses_minimal_manifest`
- `roundtrip_preserves_dev_deps`
- `resolves_caret_to_latest_compatible`
- `errors_on_missing_name_field`

Avoid `test_foo` prefixes — `cargo test` already labels them as tests, and
the prefix bloats output.

## Test scope

Tests must never hit the network. Not "usually never," not "only in
`#[ignore]` tests" — never. CI runs without network access and a flaky
network is the single largest source of test pain in package-manager
projects.

Build fixtures inline:

- Small JSON literals as raw string literals.
- In-memory tarballs via `tar::Builder` writing into a `Vec<u8>`, optionally
  wrapped in `flate2::write::GzEncoder` for `.tgz` payloads.

```rust
use flate2::Compression;
use flate2::write::GzEncoder;
use tar::Builder;

fn make_tgz(files: &[(&str, &[u8])]) -> Vec<u8> {
    let buf = Vec::new();
    let enc = GzEncoder::new(buf, Compression::default());
    let mut tar = Builder::new(enc);
    for (path, data) in files {
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, path, *data).unwrap();
    }
    tar.into_inner().unwrap().finish().unwrap()
}
```

If a unit needs an HTTP client, take a trait or a closure so tests can pass a
canned implementation. Don't reach for `mockito` or similar — the indirection
is rarely worth it at this size.

## Disk side-effects

Anything that writes to disk uses `tempfile::TempDir`. Never read from or
write to `~/.guroku` from a test. There is no opt-out for this rule.

For the content-addressed store:

```rust
use tempfile::TempDir;

#[test]
fn extracts_tarball_into_cas() {
    let tmp = TempDir::new().unwrap();
    let bytes = make_tgz(&[("package/index.js", b"module.exports = 1;")]);
    let path = store::ensure_extracted_at(tmp.path(), &bytes).unwrap();
    assert!(path.join("index.js").exists());
}
```

For the metadata cache, use the `_in` variants that take an explicit root:

```rust
let tmp = TempDir::new().unwrap();
http_cache::write_in(tmp.path(), "lodash", &meta_bytes).unwrap();
let round = http_cache::read_in(tmp.path(), "lodash").unwrap();
assert_eq!(round, meta_bytes);
```

If you find yourself wanting to test something that only takes a
"home directory" implicitly, that's a signal: the function under test should
be refactored to accept a root path.

## Asserting on paths

`Path::ends_with` matches whole path components, not string suffixes. This
catches people regularly:

```rust
let p = Path::new("/tmp/abc/lodash-4.17.21");
assert!(p.ends_with("lodash-4.17.21"));   // OK — whole component
assert!(!p.ends_with("4.17.21"));          // false — not a component
```

For substring checks (e.g. "the path contains the version string somewhere"),
convert to a string:

```rust
assert!(p.to_string_lossy().contains("4.17.21"));
```

Prefer component-aware assertions where possible — they fail more clearly
when something moves.

## Fixtures

JSON fixtures live at `tests/fixtures/`. Use them when the data is too large
to inline comfortably — registry metadata for a real package, or a
`package.json` with many fields. Smaller fixtures are better inlined; an
inlined literal next to the assertion is easier to read than a
`include_str!` redirect.

```rust
const MANIFEST: &str = include_str!("fixtures/lodash-4.17.21.json");

#[test]
fn parses_real_world_manifest() {
    let m = manifest::parse(MANIFEST).unwrap();
    assert_eq!(m.name, "lodash");
}
```

Keep fixtures stable: do not regenerate them on every test run, and do not
edit them by hand to fit a new test — write a new fixture instead.

## Async tests

Use `#[tokio::test]` if the function under test is async. Most v0.1-v0.3
tests are sync-friendly because the units involved (parsing, path
computation, semver matching) don't need a runtime.

```rust
#[tokio::test]
async fn fetches_metadata_via_injected_client() {
    let client = FakeClient::new(canned_response());
    let meta = registry::fetch_with(&client, "lodash").await.unwrap();
    assert_eq!(meta.name, "lodash");
}
```

Don't spin up a runtime by hand inside a `#[test]` — `#[tokio::test]` is
clearer and configures the runtime correctly.

## CLI tests

CLI tests live in `tests/cli_*.rs` and drive the built binary directly:

```rust
use std::process::Command;
use tempfile::TempDir;

#[test]
fn install_creates_node_modules() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("package.json"),
        r#"{"name":"x","version":"0.1.0"}"#).unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_guroku"))
        .args(["--cwd"])
        .arg(tmp.path())
        .arg("install")
        .output()
        .unwrap();

    assert!(out.status.success(), "stderr: {}",
        String::from_utf8_lossy(&out.stderr));
    assert!(tmp.path().join("node_modules").exists());
}
```

Always pass `--cwd <tempdir>`. Never let a CLI test default to the
process working directory — that pollutes the project checkout and
makes tests order-dependent.

## Running tests

A single test file:

```sh
cargo test --test lockfile_roundtrip
```

A single test function (matches by substring across all binaries):

```sh
cargo test parses_minimal_manifest
```

Show output for passing tests too:

```sh
cargo test -- --nocapture
```

## Running tests in CI

The `ci` workflow runs:

```sh
cargo test --all
```

This compiles and runs every test binary across the workspace. Keep
individual binaries cheap; if you're tempted to add a slow test, see if it
can be a unit test instead.

## Coverage

The coverage workflow runs:

```sh
cargo llvm-cov --workspace --lcov --output-path lcov.info
```

You can run the same command locally if you have `cargo-llvm-cov`
installed. We don't enforce a coverage threshold — coverage is a tool for
finding holes, not a target to game.

## Adding a new test file

Drop a new file under `tests/`. No `Cargo.toml` changes needed; cargo
auto-discovers `tests/*.rs`.

If your new test needs a dev-dependency that isn't already there, edit
`[dev-dependencies]` in `Cargo.toml`. Be parsimonious: the current dev
dependencies are `tempfile` and `criterion`, and we'd like to keep that list
short. Before adding a new one, ask whether you can do it with what's already
there or with the standard library.

## What NOT to test

The following are caught by other tools in CI; do not write tests for them:

- Clippy lints — caught by `cargo clippy -- -D warnings`.
- Formatting — caught by `cargo fmt --check`.
- Build success — caught by `cargo build` and by `cargo test` itself.

If your test is "this compiles," delete it. If your test is "clippy is
happy," delete it. If your test is "rustfmt didn't reformat this," delete it
twice.

## Flaky-test policy

Flake means "fix or delete." A test that occasionally fails is worse than no
test: it teaches the team to ignore red CI, which is the worst habit a
project can develop.

If a test flakes:

1. First, try to reproduce it. Run it in a loop:
   ```sh
   for i in $(seq 1 100); do cargo test --test the_flaky_one || break; done
   ```
2. If you find the cause (timing assumption, ordering assumption, shared
   state), fix it.
3. If you cannot fix it within a day, delete it and open an issue. The suite
   is more valuable trustworthy-and-smaller than untrustworthy-and-bigger.

Do not mark tests `#[ignore]` to silence a flake. `#[ignore]` is for tests
that are slow or environment-dependent on purpose, not a graveyard.
