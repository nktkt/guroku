## Summary
(1-3 sentences.)

## Affected modules
- [ ] `src/registry.rs`
- [ ] `src/npmrc.rs`
- [ ] `src/http_cache.rs`
- [ ] `src/audit.rs`
- [ ] `src/specs.rs` / `src/git.rs` (registry-adjacent)

## Auth and credentials
- [ ] No new code path that logs raw credentials.
- [ ] No new hard-coded URLs that bypass `RegistryClient::registry_for`.
- [ ] No new HTTP path that skips `RegistryClient::auth_for`.
- [ ] If adding a new registry-call site, it goes through `RegistryClient` rather than reqwest directly.

## Compatibility
- [ ] `Npmrc` parsing still tolerates the legacy keys we don't read (no panic).
- [ ] Behaviour with no `.npmrc` files unchanged.
- [ ] Behaviour with the public default `https://registry.npmjs.org` unchanged.
- [ ] Per-scope routing still picks `<scope>:registry=` over the default base.

## Self-hosted registry impact
- [ ] Tested against (at least mentally walked through) Verdaccio / Artifactory / Nexus / GitHub Packages where reasonable.
- [ ] Audit endpoint behaviour considered (many self-hosted servers don't proxy `/-/npm/v1/security/advisories/bulk`).

## Testing
- [ ] Tests added under `tests/registry_*` or `tests/npmrc_*` or `tests/audit_*`.
- [ ] `cargo test --all` passes.
- [ ] `cargo clippy --all-targets -- -D warnings` clean.
- [ ] `cargo fmt --check` clean.
- [ ] If a network code-path changed, ran a manual install against the public registry once.

## Documentation
- [ ] CHANGELOG.md `[Unreleased]` updated.
- [ ] `docs/auth.md` / `docs/private-registries.md` / `docs/audit.md` updated when user-visible behaviour changes.
- [ ] `docs/internals/auth.md` / `docs/internals/private-registries.md` / `docs/internals/audit.md` updated when implementation changes.

## Risk and rollback
(1-3 sentences. Be honest about credentials handling.)
