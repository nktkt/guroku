# Glossary

A reference for terms that show up across guroku's documentation, source, and
issue tracker. Definitions are scoped to how guroku uses each word; the broader
npm ecosystem may use them slightly differently.

Terms are grouped by topic. Where a deeper treatment exists, the entry links
to the relevant document.

## Storage and identity

- **CAS** — Content-Addressable Store. The on-disk pool where guroku keeps the
  unpacked contents of every tarball it has ever fetched, addressed by the
  hash of the tarball rather than by name and version. The layout is
  `~/.guroku/cas/<sha[0:2]>/<sha[2:]>/`, which keeps directory fan-out
  manageable on filesystems that dislike huge flat directories. See
  [storage.md](./storage.md).

- **CAS marker** — A zero-byte `.guroku-cas-ready` file written into a CAS
  entry once extraction has finished and every file is in place. guroku checks
  for the marker before reusing a CAS entry; an entry without a marker is
  treated as partial and re-fetched. The marker exists so that a crash
  mid-extraction cannot leave a half-populated directory that later runs
  mistake for a complete one. See [storage.md](./storage.md).

- **CAS hash** — The SHA-512 of the raw npm tarball bytes, used as the key
  under which a package's contents are stored in the CAS. Because the hash is
  computed over the tarball rather than the unpacked tree, two packages that
  publish identical bytes share a single CAS entry, and guroku can verify
  integrity before unpacking a single file.

- **store** — The older v0.1/v0.2 layout at `~/.guroku/store/<name>/<version>`,
  in which each package version had its own directory keyed by name and
  version rather than by content hash. The store is unused in v0.3 and later;
  the CAS replaces it. Old store directories are safe to delete.

- **integrity** — The per-tarball checksum that the npm registry publishes
  alongside each version, formatted as `sha512-<base64>`. guroku recomputes
  the hash of each downloaded tarball and compares it against the integrity
  field before extracting anything; a mismatch aborts the install.

## Layout

- **strict layout** — guroku v0.3+ `node_modules` shape, in which every
  package version lives once at `node_modules/.guroku/<name>@<version>/...`
  and is exposed to consumers via symlinks. Borrowed from pnpm. The strict
  layout is what makes phantom dependencies impossible: a package can only
  resolve a name if it explicitly depends on it. See
  [internals](./internals/).

- **flat layout** — The npm-style shape, in which every transitive dependency
  is hoisted to the top level of `node_modules`. This is what npm and yarn
  classic produce. guroku does not produce a flat layout in v0.3, and there
  are no plans to add one.

- **`.guroku/`** — The directory inside `node_modules` that holds the
  materialised package directories under the strict layout. Functionally
  equivalent to pnpm's `.pnpm/`. Tools that crawl `node_modules` should
  generally treat `.guroku/` as opaque.

- **surface symlink** — A top-level `node_modules/<name>` symlink, created
  for each direct dependency of the project, pointing into the corresponding
  `.guroku/<name>@<version>/node_modules/<name>` directory. Surface symlinks
  are what make `require('foo')` resolve from project code.

- **sibling symlink** — A symlink inside a package's own `node_modules/`
  directory pointing at one of its dependencies' materialised package
  directories. Sibling symlinks are how a transitive dependency reaches its
  own dependencies without anything being hoisted.

## Linking

- **hardlink** — A directory entry pointing to the same inode as another
  directory entry; the two entries are indistinguishable on disk and share
  bytes. guroku uses hardlinks from the CAS into materialised package
  directories so that thousands of installs share one copy of each file.

- **symlink** — A file or directory whose contents are a path string that the
  operating system resolves on access. guroku uses symlinks between
  `.guroku/` package directories and as surface entries; symlinks, unlike
  hardlinks, can cross filesystem boundaries and target directories.

- **junction** — A Windows-specific construct that behaves like a directory
  symlink for some purposes but is implemented differently. guroku does not
  use junctions; on Windows it relies on real symlinks, which require
  developer mode or appropriate privileges.

## Resolution

- **manifest** — The `package.json` of a project or package, parsed for its
  `dependencies`, `devDependencies`, and related fields. The manifest
  declares what is wanted; the lockfile records what was chosen.

- **lockfile** — `guroku.lock`, the file in which guroku records the exact
  resolved version of every dependency, direct or transitive, along with its
  integrity, resolved URL, and dependency edges. See
  [lockfile-format.md](./lockfile-format.md).

- **resolver** — The breadth-first solver in `src/resolver.rs` that turns
  a manifest plus the registry's metadata into a fully pinned dependency
  graph. The resolver's output is what gets written to the lockfile.

- **range** / **spec** — An npm semver constraint string such as `^1.2.3`,
  `~1.0`, `>=1 <2`, `1.x`, or `^1 || ^2`. Ranges appear in manifests; the
  resolver picks a concrete version that satisfies every range that names
  the package along the path.

- **dist-tag** — A named pointer (`latest`, `next`, `beta`, ...) maintained
  by the registry into one of a package's published versions. `guroku add`
  with no version specifier resolves through the `latest` dist-tag.

- **resolution conflict** — The case where two paths through the dependency
  graph require ranges that have no common version of the same package
  name. guroku reports a conflict and aborts; the strict layout means it
  cannot paper over the disagreement by hoisting.

## Networking

- **registry** — The npm package server that serves metadata and tarballs.
  guroku defaults to `https://registry.npmjs.org` and accepts overrides via
  configuration for private or mirror registries.

- **ETag** — An HTTP cache validator the registry returns alongside metadata
  responses. guroku v0.3 stores ETags with cached metadata and sends them
  back as `If-None-Match` on the next request, which lets the registry skip
  retransmitting unchanged bodies.

- **304** / **Not Modified** — The HTTP response code the registry returns
  when an `If-None-Match` ETag matches what it would have served. On a 304,
  guroku reuses the cached metadata body and avoids a download.

## Dependency kinds

- **dependency** — A package listed under `dependencies` in the manifest;
  installed by guroku for both `install` and `install --production`.

- **dev dependency** — A package listed under `devDependencies`; installed
  by guroku for plain `install` but skipped for `install --production`.

- **peer dependency** — A package listed under `peerDependencies`; declared
  but NOT installed by guroku in v0.3. The resolver checks for compatibility
  among peers that happen to be installed; automatic installation of peers
  is planned for v0.4. See [peer-dependencies.md](./peer-dependencies.md).

- **optional dependency** — A package listed under `optionalDependencies`;
  declared but NOT installed by guroku in v0.3. Native add-ons that publish
  themselves as optional will not be fetched until support lands. See
  [optional-dependencies.md](./optional-dependencies.md).

- **transitive dependency** — A package brought into the graph because some
  other package declared it under its own `dependencies`, rather than
  because the project at the root did. Transitive deps live entirely under
  `.guroku/` in the strict layout.

- **diamond dependency** — The shape that arises when two paths through the
  graph converge on the same package name, possibly with different ranges.
  guroku resolves a diamond to a single version when the ranges intersect
  and to a resolution conflict when they do not.

## Operational

- **frozen lockfile** — The mode invoked by `guroku install --frozen-lockfile`,
  in which guroku refuses to refresh `guroku.lock` and aborts if the
  manifest and lockfile disagree. Intended for CI, where any drift between
  what the developer committed and what would be installed should be a
  build failure.

- **phantom dependency** — A `require` or `import` of a package that the
  caller did not declare in its own manifest, which happens to work because
  some other package caused that name to appear in `node_modules`. Phantom
  dependencies are a major source of breakage when layouts change; the
  strict layout makes them impossible because nothing the caller did not
  declare is reachable on its module path.

- **hoisting** — The practice of moving a transitive dependency up to the
  top level of `node_modules` so that more packages can resolve it from a
  single location. npm and yarn classic hoist aggressively. guroku does NOT
  hoist; every package sees only what it declared.
