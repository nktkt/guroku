# Version Resolution in guroku

This document describes how guroku turns a dependency specifier (the right-hand
side of a `package.json` entry, e.g. `"^1.2.3"`, `"latest"`, `"4.5.6"`) into a
concrete published version that can be downloaded and installed.

It is aimed at contributors working on the resolver, not end users.

## 1. Status

guroku v0.1 has **no real resolver**. What exists today is a deliberate stub
whose only job is to let the rest of the install pipeline (registry fetch,
tarball download, extraction, linking, lockfile write) be exercised
end-to-end on real packages.

In particular, v0.1 does **not** understand semver ranges. `^1.2.3` and
`~1.2.3` and `>=1.0.0 <2.0.0` are all treated identically, and not in a way
you would want in production. See section 3.

A semver-aware resolver based on PubGrub is the v0.2 milestone. See section 5.

## 2. What v0.1 actually does

The single entry point is `PackageMetadata::resolve(spec)`, where
`PackageMetadata` is the deserialised registry document for one package and
`spec` is the raw specifier string from `package.json` (or the CLI).

The algorithm, in order:

1. If `spec` matches a key in the `versions` map exactly (e.g. `"1.2.3"`
   matches the version `1.2.3`), return that version unchanged. This is the
   only path that respects the user's intent precisely.
2. Else if `spec` matches a key in the `dist-tags` map (e.g. `"latest"`,
   `"next"`, `"beta"`), follow the tag and return the version it points to.
3. Else if `spec` is the empty string `""`, `"*"`, or `"latest"`, use the
   `latest` dist-tag.
4. Else fall back to the `latest` dist-tag and emit a `tracing::warn!` so the
   user can see, in `--verbose` mode, that resolution silently degraded.

In Rust pseudocode:

```rust
impl PackageMetadata {
    pub fn resolve(&self, spec: &str) -> Result<&Version> {
        // 1. exact version match
        if let Some(v) = self.versions.get(spec) {
            return Ok(v);
        }

        // 2. dist-tag match
        if let Some(tag_target) = self.dist_tags.get(spec) {
            return self
                .versions
                .get(tag_target)
                .ok_or(ResolveError::DanglingTag);
        }

        // 3. wildcard / empty / "latest" => latest dist-tag
        if matches!(spec, "" | "*" | "latest") {
            return self.latest();
        }

        // 4. give up, fall back to latest
        tracing::warn!(
            spec = spec,
            name = %self.name,
            "no resolver for range; falling back to dist-tags.latest"
        );
        self.latest()
    }

    fn latest(&self) -> Result<&Version> {
        let target = self
            .dist_tags
            .get("latest")
            .ok_or(ResolveError::NoLatestTag)?;
        self.versions
            .get(target)
            .ok_or(ResolveError::DanglingTag)
    }
}
```

That is the whole resolver. There is no constraint solving, no backtracking,
and no awareness that `^`, `~`, `>=`, `<`, `||`, or `-` mean anything.

## 3. Why this is wrong

The fallback in step 4 is the dangerous part. Consider a user who writes:

```json
{ "dependencies": { "react": "^17.0.2" } }
```

A correct resolver would pick the highest published `17.x.y` that satisfies
`>=17.0.2 <18.0.0`. The v0.1 stub instead falls through to `dist-tags.latest`
and happily installs React 19, silently crossing two major versions.

This is the canonical way to break a project. **v0.1 is a placeholder, not a
recommendation.** While running on v0.1, users should pin exact versions in
their `package.json`:

```json
{ "dependencies": { "react": "17.0.2" } }
```

This is the only specifier form for which v0.1's behaviour is actually
correct (case 1 above).

## 4. Where the metadata comes from

For every distinct package name in the dependency graph, guroku issues a
single GET request to:

```
https://registry.npmjs.org/<name>
```

The response is the full package document: every published version, every
dist-tag, every tarball URL, every `dependencies` block, every `engines`
block, etc. We deserialise it directly into `PackageMetadata`.

A few notes on this choice:

- This is **not** the abbreviated / minified metadata format
  (`application/vnd.npm.install-v1+json`). The abbreviated form drops fields
  like `readme`, `maintainers`, and per-version `_npmUser`, which can shrink
  the payload substantially for popular packages. Switching to it is a future
  optimisation, not a v0.1 concern.
- We do not currently send `If-None-Match` / `ETag` headers, so every run
  re-downloads metadata for every package. See section 6.
- We do not currently use the `~/<name>` packument-only endpoints or the
  `-/v1/search` endpoint. Resolution does not need them.

## 5. v0.2 plan: PubGrub

For v0.2, guroku will adopt **PubGrub**, the version-solving algorithm
designed by Natalie Weizenbaum for Dart's `pub` package manager. PubGrub is
also what powers Astral's `uv` (Python) and is conceptually aligned with
Cargo's resolver. Choosing it gives us:

- Fast convergence on real-world dependency graphs.
- High-quality error messages: when resolution fails, PubGrub explains *why*
  in terms of the user's own constraints, not internal solver state.
- A mature Rust implementation in the
  [`pubgrub`](https://crates.io/crates/pubgrub) crate, which we can plug in
  rather than re-implement.

Brief sketch of how guroku will drive `pubgrub`:

1. Seed the solver with the project's direct dependencies as the root
   package's requirements.
2. For each package the solver asks about, fetch its registry metadata (with
   caching, see section 6) and adapt the `versions` + `dependencies` maps
   into `pubgrub`'s `DependencyProvider` trait.
3. The solver builds a partial solution incrementally, picking versions for
   one package at a time.
4. On conflict (e.g. package A requires `lodash@^4`, package B requires
   `lodash@^3`), PubGrub derives an *incompatibility* term, backtracks to
   the most recent decision that could have caused the conflict, and tries
   again.
5. Termination yields a flat `name@version` set. We then look up each pair
   in the cached metadata to recover the concrete tarball URL, integrity
   hash, and per-version dependency list, and hand that off to the existing
   v0.1 download/extract/link pipeline.

The interesting work in the v0.2 milestone is almost entirely the adapter
between npm semver ranges and `pubgrub`'s `VersionSet` model, plus the
metadata cache. The solver itself is library code.

## 6. Open questions for v0.2

These are real design questions, not bugs. Each has more than one defensible
answer and we want to think before committing.

- **Peer dependencies.** npm and pnpm differ on whether peers are part of the
  solve or applied after. pnpm models them in the resolver and produces
  multiple installations of the same package keyed by peer set; npm uses a
  more lenient post-pass. We need to pick one.
- **Optional dependencies and `os` / `cpu` / `libc` fields.** A package may
  publish an entry that should only be installed on, say, `linux-arm64`.
  Should the solver consider unsupported-platform versions at all? If yes,
  what does the lockfile look like across platforms?
- **Yanked and deprecated versions.** npm has no `yanked` flag in the
  Cargo/PyPI sense, but it does have `deprecated` strings on individual
  versions. Do we silently exclude them, warn, or accept them only when
  explicitly requested?
- **Caching of registry metadata between runs.** Each packument is large and
  changes rarely. We should persist them under
  `~/.cache/guroku/registry/<name>.json` together with the response `ETag`
  and revalidate with `If-None-Match` on subsequent runs. Open question: do
  we cache by package or by package-and-version, and what's the eviction
  policy?

## 7. Reference reading

- Natalie Weizenbaum, *PubGrub: Next-Generation Version Solving* —
  <https://nex3.medium.com/pubgrub-2fb6470504f>
- npm registry API notes —
  <https://github.com/npm/registry/blob/master/docs/REGISTRY-API.md>
- pnpm's resolver discussion in the pnpm docs —
  <https://pnpm.io/motivation> and the linked design notes on how pnpm
  resolves peer dependencies into distinct dependency-graph nodes.
- The `pubgrub` crate, which v0.2 will depend on —
  <https://docs.rs/pubgrub>.
