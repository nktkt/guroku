# Dependency Specs (`src/specs.rs`)

This document describes how guroku classifies the right-hand side of a
dependency entry. Given a string like `"^1.2.3"`, `"file:./vendor/foo"`, or
`"github:lodash/lodash#main"`, we need to decide whether to hand it off to the
semver resolver, the file-system path, or the git fetcher. That single
decision lives in `src/specs.rs`.

The module is intentionally small and stupid: it does not validate semver
ranges, it does not check that paths exist, and it does not try to clone
anything. It just classifies.

## 1. The `DepSpec` enum

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DepSpec {
    /// A semver range string, e.g. "^1.2.3", "~0.4", "1.x", "*", or a tag
    /// like "latest". Interpreted by the `version` module downstream.
    Range(String),

    /// A local filesystem path, e.g. "./vendor/foo" or "../sibling".
    /// Stored verbatim from the original spec, with the `file:` prefix
    /// already stripped.
    File(String),

    /// A git repository, with an optional revision (branch, tag, or sha).
    Git {
        url: String,
        revision: Option<String>,
    },
}
```

Three variants is the entire surface. Anything we cannot classify into File
or Git falls into Range and the version module has the final say.

## 2. The `classify(s)` function

`classify` is a single string in, a `DepSpec` out. It walks a short
decision tree, in order:

```rust
pub fn classify(s: &str) -> DepSpec {
    // 1. file:./path
    if let Some(rest) = s.strip_prefix("file:") {
        return DepSpec::File(rest.to_string());
    }

    // 2. git+https:// or git+ssh:// (or any git+<scheme>://)
    if let Some(rest) = s.strip_prefix("git+") {
        let (url, rev) = split_revision(rest);
        return DepSpec::Git { url: url.to_string(), revision: rev };
    }

    // 3. bare git://
    if s.starts_with("git://") {
        let (url, rev) = split_revision(s);
        return DepSpec::Git { url: url.to_string(), revision: rev };
    }

    // 4. SCP-style git@host:path
    if is_scp_git(s) {
        let (url, rev) = split_revision(s);
        return DepSpec::Git { url: url.to_string(), revision: rev };
    }

    // 5. github:user/repo[#ref]
    if let Some(rest) = s.strip_prefix("github:") {
        let (path, rev) = split_revision(rest);
        let url = format!("https://github.com/{}", path);
        return DepSpec::Git { url, revision: rev };
    }

    // 6. Fallback: treat as a semver range / tag.
    DepSpec::Range(s.to_string())
}
```

The cases, briefly:

- `file:./path` — strip the `file:` prefix and keep the rest verbatim.
  We do not normalise `./` vs no prefix, so what the user wrote is what we
  store.
- `git+https://...` / `git+ssh://...` — strip the `git+` prefix; the
  remaining string is a normal URL.
- bare `git://...` — already a git URL, no rewriting needed.
- `git@host:path` — SCP-style; detected by `is_scp_git` (a `@` before the
  first `:`, no `://`, no whitespace).
- `github:user/repo` — expanded to `https://github.com/user/repo`. This is
  the only shorthand we expand. `gitlab:`, `bitbucket:` etc. are not
  supported in v0.5.
- everything else — `Range`. The version module owns interpretation.

## 3. Revision parsing

Git specs may have a trailing `#<ref>`:

```
git+https://github.com/foo/bar.git#v1.2.3
github:foo/bar#main
git@github.com:foo/bar.git#abc123
```

The helper `split_revision` peels that off:

```rust
fn split_revision(s: &str) -> (&str, Option<String>) {
    if let Some(idx) = s.rfind('#') {
        let (head, tail) = (&s[..idx], &s[idx + 1..]);
        // Only treat the trailing fragment as a revision if it does not
        // contain a slash. This is a pragmatic compromise: query strings
        // like "?foo=bar/baz" stay on the URL side.
        if !tail.contains('/') {
            return (head, Some(tail.to_string()));
        }
    }
    (s, None)
}
```

The `/`-check is the pragmatic compromise. A strictly URL-correct parser
would tokenise scheme/authority/path/query/fragment and split only on the
fragment delimiter. We don't do that, because:

- Real-world git refs almost never contain `/` for our purposes (`main`,
  `v1.2.3`, a sha). Branches like `release/1.x` exist but are uncommon
  in package manifests.
- Query strings with `/` inside values do show up (auth tokens, paths).
- This matches npm's behaviour for the cases users actually hit.

If a user really has a `release/1.x` branch, they can pin the SHA or use
the long-form git URL with their own ref-spec mechanism. We accept the
edge-case loss.

## 4. `unparse(&DepSpec)`

The inverse of `classify`. Used when serialising back to lockfiles or
manifests.

```rust
pub fn unparse(spec: &DepSpec) -> String {
    match spec {
        DepSpec::Range(s) => s.clone(),
        DepSpec::File(path) => format!("file:{}", path),
        DepSpec::Git { url, revision } => {
            let prefixed = if url.starts_with("git+") {
                url.clone()
            } else {
                format!("git+{}", url)
            };
            match revision {
                Some(r) => format!("{}#{}", prefixed, r),
                None => prefixed,
            }
        }
    }
}
```

Notes:

- `Range` round-trips byte-for-byte.
- `File` round-trips byte-for-byte (we re-add `file:`).
- `Git` always emits a `git+` prefix in the canonical form, even if the
  user originally wrote `github:foo/bar` or a bare `git://...`. This is
  intentional: the canonical form is unambiguous.

## 5. `validate(&DepSpec)`

Currently a placeholder:

```rust
pub fn validate(_spec: &DepSpec) -> Result<(), SpecError> {
    Ok(())
}
```

This exists so callers can adopt the validation point now and get
real checks for free in v0.6. Planned uses:

- Reject `workspace:*` once the workspace protocol lands (so we can fail
  loudly on workspaces in non-workspace projects).
- Warn on `file:` paths that escape the project root.
- Warn on git URLs missing a pinned revision.

For v0.5, every `DepSpec` is valid.

## 6. What we don't yet support

The following spec syntaxes parse as `Range` today, which is wrong but
non-fatal — the version module will fail to resolve them with a clear
error. They are scheduled for later versions:

- `npm:<alias>@<spec>` — npm aliases. Needs a fourth `Alias { name, spec }`
  variant.
- `workspace:*` / `workspace:^` / `workspace:~` — workspace-protocol
  specs. Requires the workspace resolver (see
  `docs/internals/workspaces.md`).
- `link:./path` — yarn-style symlink installs. Conceptually similar to
  `File` but with different install semantics (live link vs. copy).
- Raw URLs to tarballs (e.g. `https://example.com/foo-1.0.0.tgz`).
  Will need a `Tarball { url, integrity }` variant.

If you hit one of these, classification will silently route it to the
range resolver, which will reject it. That's the right failure mode for
v0.5: explicit error at the resolution stage, not silent breakage.

## 7. How the resolver consumes `DepSpec`

The BFS resolver (see `docs/internals/algorithm-notes.md`) branches on
the spec enum and dispatches to the appropriate fetcher:

```rust
match classify(&dep.spec) {
    DepSpec::Range(r)              => registry.resolve(&dep.name, &r),
    DepSpec::File(path)            => fs_resolver.resolve(&path),
    DepSpec::Git { url, revision } => git_resolver.resolve(&url, revision.as_deref()),
}
```

Each branch returns the same `Resolved` shape, so the rest of the
pipeline (lockfile, CAS, install) is spec-agnostic.

## 8. Round-trip safety

`classify(unparse(x)) == x` holds for `Range` and `File` trivially, since
both store their payload verbatim. For `Git`, the canonical form is
`git+<scheme>://...[#rev]`, which `classify` parses back into the same
`Git { url, revision }` value.

Round-trip is **not** preserved for the input form. Specifically:

- `github:foo/bar` becomes `git+https://github.com/foo/bar` after a
  round trip.
- `git://example.com/foo.git` becomes `git+git://example.com/foo.git`.

This is fine: the lockfile stores the canonical form, and we never need
to reproduce the user's original shorthand.

A small smoke test:

```bash
$ cargo test -p guroku --lib specs::round_trip
running 4 tests
test specs::round_trip::range ... ok
test specs::round_trip::file  ... ok
test specs::round_trip::git_https ... ok
test specs::round_trip::git_with_rev ... ok
```

That suite locks the canonicalisation behaviour. If we ever change the
canonical form, those tests will need updating in lockstep with a
lockfile schema bump.
