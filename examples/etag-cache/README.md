# etag-cache

A README-only example illustrating guroku v0.3's ETag-aware metadata cache.

## What this example shows

How to verify that re-installing a project a second time hits the ETag cache
(304 Not Modified) instead of pulling the full registry document. Guroku
persists the registry response body alongside the `ETag` header it received,
and on the next install it sends `If-None-Match: <etag>` so the registry can
short-circuit with a 304 when nothing has changed.

This is mostly useful for:

- Repeated installs in CI where the lockfile is already authoritative but
  metadata still needs to be revalidated.
- Local development loops where you re-run `guroku install` after editing
  `package.json`.

## Setup

Create a tiny project with a single dependency that has a stable, well-known
metadata document:

```sh
mkdir -p /tmp/etag-demo && cd /tmp/etag-demo
echo '{"name":"etag-demo","version":"0.1.0","dependencies":{"is-odd":"^3.0.0"}}' > package.json
```

## First install

The first install has no cached metadata, so guroku pulls each registry
document fresh:

```sh
GUROKU_LOG=debug guroku install 2>&1 | grep -i "metadata"
```

Expect log lines about fetching `is-odd` metadata (and any of its transitive
deps). At this point the cache is being populated.

## Where the cache lives

```sh
ls ~/.guroku/cache/metadata
# is-odd.json
# is-odd.etag
cat ~/.guroku/cache/metadata/is-odd.etag
# "<some-etag-string>"
```

The `.json` file holds the body of the packument; the `.etag` file holds the
exact ETag header value the registry returned (quotes preserved, since the
spec treats them as part of the opaque token).

## Second install

Run the same command again. This time guroku attaches `If-None-Match` to each
metadata request. A healthy registry will respond `304 Not Modified` and
guroku will reuse the on-disk body:

```sh
guroku install
# Notice fewer/no metadata fetch log lines.
```

With `GUROKU_LOG=debug`, look for the line:

```
metadata cache hit (304) for is-odd
```

If you instead see `metadata cache miss` or a full `200 OK` log line for a
package whose `.etag` exists, that usually means the registry rotated its
ETag (e.g. a new version was published) -- guroku will transparently fall
back to the fresh body.

## Force a refetch

Drop the cache directory if you want guroku to revalidate everything from
scratch:

```sh
rm -rf ~/.guroku/cache/metadata
guroku install
```

Or, from the library API, disable the HTTP cache for one client:

```rust
RegistryClient::with_default_registry()?.without_http_cache()
```

## What's cached

- Registry metadata bodies (`<pkg>.json`).
- The `ETag` header that came with each body (`<pkg>.etag`).

What is NOT cached at the HTTP layer:

- Tarballs. They go through the content-addressed store (CAS) instead, keyed
  by the integrity hash from the packument, so an HTTP cache would be
  redundant.

## What's NOT yet supported

- `Cache-Control: max-age` -- guroku always re-validates with the registry
  on every install rather than serving a cached body without a conditional
  request.
- `Vary` headers -- the cache key is just the package name, so registries
  that vary responses by `Accept` or auth headers may produce surprising
  results.
- Request coalescing -- if two parallel resolver tasks both ask for the same
  package at the same time, two requests go out. A future version will
  collapse them into a single in-flight future.

## Related docs

- `docs/internals/http-cache.md`
