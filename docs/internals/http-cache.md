# HTTP Metadata Cache (v0.3)

This document describes guroku's ETag-aware HTTP cache for npm registry
metadata. It is an internals note: callers of the public CLI should not
need to know any of this, but contributors working on the resolver or the
registry client will.

## What's cached

The cache stores npm registry **metadata** responses --- specifically,
the JSON body returned by `GET https://registry.npmjs.org/<name>`. These
documents describe a package: its versions, dist-tags, dependency
declarations per version, tarball URLs, integrity strings, and so on.

Tarballs are explicitly **not** cached at the HTTP layer. Tarballs flow
through the content-addressable store (CAS) instead, which keys them by
the SHA-512 integrity hash declared in the metadata. The CAS is more
durable than HTTP cache headers (we trust the hash, not the server's
freshness signals) and is shared across packages and projects, so the
same tarball downloaded for two projects is stored exactly once.

In short:

- Metadata --- HTTP cache (this document).
- Tarballs --- CAS (`docs/internals/cas.md`).

## Where it lives

Cache files live under `~/.guroku/cache/metadata/`. For a package named
`<name>` we write two files:

```
~/.guroku/cache/metadata/<name>.json    # the response body
~/.guroku/cache/metadata/<name>.etag    # the ETag header value, if any
```

Scoped names need a small massaging step because `/` is a path
separator. We replace it with `+`, so `@types/node` is stored at:

```
~/.guroku/cache/metadata/@types+node.json
~/.guroku/cache/metadata/@types+node.etag
```

This mirrors the convention pnpm uses for its own metadata cache and has
the nice property of being reversible without ambiguity (npm names
never contain `+`).

The `.etag` file is plain text: just the raw header value, no quoting
massage, no JSON envelope. If the registry ever returned an ETag with
trailing whitespace we'd preserve it byte-for-byte, which is what we
want for a header that goes back over the wire untouched.

## The conditional-GET dance

Every call to `RegistryClient::fetch_metadata(name)` does the following:

1. Look up the cached body and ETag for `<name>`.
2. If an ETag is present, attach `If-None-Match: <etag>` to the outgoing
   request.
3. If the server responds with `304 Not Modified`, return the cached
   body unchanged. The body never crosses the wire.
4. If the server responds with `200 OK`, replace the cache with the new
   body. If the response carries a new `ETag` header, write that too;
   if it doesn't, delete the stale `.etag` file so we don't accidentally
   send a stale validator next time.
5. Any other status (4xx, 5xx) is propagated to the caller as an error.
   The cache is left as-is on error --- a flaky registry shouldn't
   nuke a perfectly good cached body.

In Rust pseudocode:

```rust
pub async fn fetch_metadata(&self, name: &str) -> Result<Metadata> {
    let cached = http_cache::read_in(&self.cache_dir, name)?;

    let mut req = self.client.get(self.registry_url(name));
    if let Some((_, etag)) = &cached {
        req = req.header("If-None-Match", etag);
    }

    let resp = req.send().await?;
    match resp.status() {
        StatusCode::NOT_MODIFIED => {
            let (body, _) = cached.expect("304 implies we sent If-None-Match");
            Ok(serde_json::from_slice(&body)?)
        }
        StatusCode::OK => {
            let new_etag = resp
                .headers()
                .get("etag")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_owned());
            let body = resp.bytes().await?;
            http_cache::write_in(&self.cache_dir, name, &body, new_etag.as_deref());
            Ok(serde_json::from_slice(&body)?)
        }
        other => Err(Error::Registry(other)),
    }
}
```

The `.expect("304 implies ...")` is correct: a compliant server only
sends 304 in response to a conditional request, and we only send a
conditional request when we have a cached body to validate against.

## What 304 saves us

npm registry "full package" documents are large. A full document for a
heavily-versioned package contains every published version's manifest,
plus dist metadata, plus README, plus maintainer info. For packages
like `lodash`, `react`, or `@babel/core` the document weighs in at
several megabytes; for the very largest packages it crosses 10 MB and
keeps growing with every release.

A `304 Not Modified` response, by contrast, is a few hundred bytes ---
just status line and headers, no body. On a typical install where most
dependencies haven't published a new version since the last resolve,
the cache turns what would be tens or hundreds of MB of metadata
transfer into a handful of conditional GETs that mostly come back
empty.

This is the single biggest reason `guroku install` on an unchanged
project feels close to instant once warmed up: the resolver is doing
network round-trips, but it's barely transferring any bytes.

## Disabling the cache

The library API exposes an opt-out:

```rust
let client = RegistryClient::without_http_cache();
```

This builds a client whose `fetch_metadata` skips both the read and the
write paths --- every call goes to the network and returns the live
response. We use this in two places today:

- **Tests** that want deterministic network behaviour and don't want to
  reason about pre-existing cache state.
- **CLI flag** (planned, not yet exposed in v0.3) for the rare case
  where a user suspects cache corruption and wants to force a clean
  fetch without manually clearing `~/.guroku/cache/metadata/`.

The opt-out is total: we don't read, we don't write. There's no
`fetch_metadata_force` variant on the cached client because the
expectation is that you know at construction time whether you want a
caching client or a non-caching one.

## What we don't cache

A few things are deliberately outside the HTTP cache's responsibility:

- **Tarballs.** The CAS handles these by content-hash. HTTP-level
  caching by URL is strictly worse than content-addressed storage for
  immutable, hash-pinned blobs --- the tarball at a given URL is
  expected to never change, so an ETag round-trip would just be
  overhead.
- **Search results** (`/-/v1/search`). These are intrinsically
  freshness-sensitive and the use case (interactive `guroku search`)
  doesn't benefit much from a stale cache. Skipped for now.
- **Auth-walled responses.** v0.3 only talks to the public npm
  registry. Private registries with auth headers will need careful
  handling --- we'd want to key cache entries per-credential to avoid
  leaking metadata across users on a shared machine. That's deferred to
  v0.5 along with the rest of private-registry support.

## Cache-Control versus ETag

npm registry responses include both `ETag` and `Cache-Control` headers.
We honour `ETag` and largely ignore `Cache-Control`.

The reasoning: `Cache-Control: max-age=N` would let us skip the network
round-trip entirely for `N` seconds after a fetch, but the values npm
serves are short and conservative, and the resolver's correctness story
is much simpler if we treat metadata as always-conditional. Every
resolve issues a conditional GET; either we get a fast 304 or we get
the new body. The user never sees stale data because we never trust
freshness without asking the server.

The cost is that you'll always hit the network at least once per
package per resolve, even immediately after a previous resolve. The
benefit is small: 304 responses are cheap and the latency is dominated
by TLS setup, which the HTTP/2 connection pool amortises across all
requests in a single resolve.

If we ever change our minds, the place to add `Cache-Control` handling
is `http_cache::should_revalidate`, which today just returns `true`
unconditionally.

## Stale-while-revalidate semantics

None today. Every cached 200 response is re-validated on next access
--- there is no "serve stale, refresh in background" path.

This has been considered for a follow-up. The shape would be:

1. On read, return the cached body immediately.
2. Spawn a background task that issues the conditional GET and updates
   the cache when it completes.
3. The next resolve sees the fresh data.

The complication is correctness: if the SWR refresh races with the
next resolve, that resolve might still pick up the old data, defeating
the point. Doing it well needs either a per-package lock or an
explicit "wait for in-flight refresh" path, and neither is free. We
shipped v0.3 without it and will revisit if profiling shows the
conditional GET round-trips dominating resolve time.

## Test surface

The `http_cache` module's read/write helpers take a directory parameter
explicitly:

```rust
pub fn read_in(dir: &Path, name: &str) -> Result<Option<(Vec<u8>, String)>>;
pub fn write_in(dir: &Path, name: &str, body: &[u8], etag: Option<&str>);
```

Tests construct a `tempfile::TempDir` and pass it in, so nothing ever
touches `~/.guroku/cache/metadata/` during a test run. The production
caller (`RegistryClient`) wraps these with the real cache directory
resolved from `$HOME` at construction time.

This is the only reason the helpers take a directory parameter rather
than reading it from a global. It also makes it easy to write tests
that exercise cache eviction, ETag mismatches, or partially-written
cache state, all without process-level locking or environment
mutation.

## Failure modes

The cache is advisory, not authoritative. We treat read and write
errors asymmetrically:

- **Write errors** are demoted to `tracing::debug!` and swallowed. If
  the disk is full, or the cache directory is read-only, or an
  antivirus is holding our handle, we still want the resolve to
  succeed --- we just won't have a cache entry next time. The trade-off
  is that a persistent write failure is silent at the user level; you
  have to be running with `RUST_LOG=guroku=debug` to see it. That's
  fine for an advisory cache, less fine for a primary store, which is
  why we structure it this way.
- **Read errors** are mostly bubbled up. A missing body file is not an
  error --- `read_in` returns `Ok(None)` and the caller falls through
  to an unconditional GET. But an existing body file that fails to
  read (permission denied, I/O error mid-read) is propagated, because
  silently treating it as "no cache" would mask a real problem and
  cause every resolve to re-download every full document.
- A missing `.etag` alongside an existing `.json` is fine: we just send
  the GET without an `If-None-Match` header. The server returns 200,
  we overwrite the body, and life goes on.
- A present `.etag` alongside a missing `.json` is treated as no-cache:
  we don't send the validator (we have nothing to fall back to on 304)
  and we tidy up the orphan etag file on the next successful write.

The general principle: this cache is a best-effort optimisation. The
correctness of every resolve must hold even if the cache directory is
deleted between calls.
