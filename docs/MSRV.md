# MSRV Policy

This document describes guroku's Minimum Supported Rust Version (MSRV)
policy: what it is, how we change it, and how downstream consumers can
rely on it.

## What MSRV is

MSRV stands for **Minimum Supported Rust Version**. It is the oldest
Rust toolchain version on which the crate is guaranteed to:

- compile cleanly with `cargo build`,
- pass the full test suite under `cargo test --all`.

If a release of guroku claims an MSRV of `X.Y`, then a user with the
`X.Y` toolchain installed should be able to build and test the crate
without reaching for a newer compiler. Anything older than the declared
MSRV is unsupported; it may happen to work, but we will not accept bug
reports against it.

MSRV is a *floor*, not a ceiling. The crate is also expected to build
and test on every stable Rust release newer than the floor, including
`stable`, `beta`, and (best-effort) `nightly`.

## The current MSRV

The current MSRV for guroku is **Rust 1.75**.

This is recorded in two places:

- `Cargo.toml`, in the `[package]` table, as `rust-version = "1.75"`.
- The CI matrix in `.github/workflows/msrv.yml`, which builds and tests
  against Rust 1.75 on every push and pull request.

If the value in `Cargo.toml` and the value in the CI workflow ever
disagree, the workflow is the source of truth and `Cargo.toml` is the
bug.

## Bump policy

Bumping the MSRV is a routine maintenance event, not a breaking change.
The rules are:

- **MSRV bumps are NOT considered breaking changes** under semver.
  Downstream crates depending on guroku do not get a major-version
  bump just because we moved off an old compiler.
- **Bumps are announced ahead of time.** See the next section for the
  announcement mechanism.
- **The deprecation window is six months** from the date of the
  announcement to the date of the release that requires the new
  toolchain. This gives embedded users, distros, and corporate
  toolchain teams time to schedule the upgrade.
- **Bumps land in MINOR releases only** (v1.x.0). They never land in
  patch releases (v1.x.y where y > 0). Patch releases are reserved for
  bug fixes that compile on the existing MSRV.
- **Each bump is documented in `CHANGELOG.md`** under the affected
  release's `### Changed` section, with the old and new MSRV values
  spelled out explicitly.

## How a bump is announced

When the maintainers decide to bump the MSRV, two things happen at
least six months before the release that ships the bump:

1. An issue is filed in the repository with the `msrv` label. The
   issue title follows the form `MSRV bump: 1.A -> 1.B (target
   release: vX.Y.0)`. The issue body includes the rationale, the
   target release version, and the target release date.
2. The next release notes published (typically the most recent minor
   release before the bump) include a "Heads up" section pointing at
   that issue.

Anyone subscribed to releases or filtering issues by the `msrv` label
will see the bump coming. We do not announce bumps over chat, social
media, or out-of-band channels; the issue and the release notes are
authoritative.

## Why we have an MSRV at all

It is tempting to just track stable Rust and call it a day. We do not,
because real users are not always on stable's edge:

- **Embedded users** often pin to a specific toolchain that has been
  validated against their hardware, RTOS, or certification process.
  Moving forward costs them re-validation effort.
- **Linux distros** (Debian stable, RHEL, Alpine, NixOS releases) ship
  a Rust toolchain that lags current stable, sometimes by a release
  or two. Distro packagers cannot upgrade Rust on a whim; their
  packaging cycle is measured in months.
- **Corporate Rust forks** (internal toolchains at large engineering
  orgs) tend to follow stable but with their own staging delay. If
  guroku required a Rust released last week, those teams would have
  to wait.

Pinning to the bleeding edge of stable would lock these users out for
no good reason. A documented, stable MSRV with a predictable bump
cadence is a small cost for us and a large benefit for them.

## What "supported" means in practice

When we say guroku supports Rust 1.75, we mean specifically:

- `cargo build` succeeds on Rust 1.75 against a `Cargo.lock` produced
  by a recent toolchain (current stable at the time of the release).
- `cargo test --all` succeeds on Rust 1.75 against the same
  `Cargo.lock`.

We do **not** promise the following:

- That `cargo update` on MSRV produces a working build. Newer
  dependency releases may themselves have moved their MSRV forward.
- That every dev-dependency builds on MSRV. Dev-dependencies are
  used for testing, benchmarking, and tooling; if a dev-dep bumps
  its own MSRV ahead of ours, we may pin it to the last
  MSRV-compatible release in `Cargo.toml` rather than follow it.
- That examples, fuzz targets, or auxiliary binaries (under
  `examples/`, `fuzz/`, `xtask/`, etc.) build on MSRV. Those are
  developer tooling and may use newer compiler features.

The contract is: take a release tag, take its `Cargo.lock`, install
the declared MSRV, and `cargo build` plus `cargo test --all` work.

## Why 1.75 specifically

Rust 1.75 stabilized in late 2023. Picking it as our floor gives us
two things we want without giving up too many users. First, it
stabilized `async fn in trait` (return-position `impl Trait` in trait
methods, more precisely), which we expect to lean on in upcoming
solver work around pubgrub follow-ups and async resolver hooks.
Second, it bundles a number of quality-of-life improvements (better
const evaluation, improved diagnostics, improved `Option` and
`Result` ergonomics in const contexts) that simplify code we would
otherwise write awkwardly. By the time guroku v1.0 shipped, 1.75 had
been stable long enough that distros and corporate toolchains had
mostly caught up, so the cost to downstream users was low. The
rationale is informational, not load-bearing; future bumps will pick
their version on the same balance of compiler features versus
downstream pain.

## What 2.0 might do

A major version bump is the natural moment to reset MSRV expectations.
When guroku v2.0 ships, we expect to:

- bump the MSRV to whichever Rust release is current stable at the
  time v2.0 is cut, or possibly one release behind it,
- restart the six-month deprecation window from that new floor.

In other words, a major bump does not commit us to dragging the v1
MSRV forward into v2. v1.x will continue to honor its own MSRV
policy on its own release branch for as long as that branch is
maintained.

## Impact on dependency choices

The MSRV policy shapes which crates we are willing to depend on:

- **Preferred:** crates whose own declared MSRV is at or below ours.
  These are safe to take a runtime dependency on without further
  thought.
- **Acceptable with care:** crates whose MSRV is newer than ours, but
  where a slightly older release of the same crate is MSRV-compatible
  and still maintained. We pin to that older release in `Cargo.toml`
  and revisit when we bump.
- **Acceptable behind a feature gate:** crates whose MSRV is newer
  and where the functionality is optional. We can put the dependency
  behind a non-default feature so MSRV users simply do not enable it.
- **Avoid:** crates whose MSRV is ahead of ours, are not pinnable to
  an older compatible release, and provide functionality we cannot
  feature-gate. In practice this means we either reimplement the
  small piece we need, wait for a backport, or defer the work until
  the next MSRV bump.

When in doubt, check the candidate crate's `Cargo.toml` for its
`rust-version` field and its CI matrix.

## Verifying locally

To reproduce what CI checks, install the MSRV toolchain and run the
build and test on it explicitly:

```sh
rustup toolchain install 1.75
cargo +1.75 build
cargo +1.75 test --all
```

The CI workflow in `.github/workflows/msrv.yml` runs the same three
commands in the same order against a clean checkout. If your local
run passes and CI fails, suspect environmental drift first
(`Cargo.lock` differences, locally-installed system libraries, or
a stale `target/` directory) before suspecting the workflow.

To pin a temporary local override for an entire working tree, use a
`rust-toolchain.toml` file at the repo root. We do not commit one,
because doing so would force every contributor onto MSRV; it is up
to the individual developer.
