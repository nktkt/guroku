# Content-Addressable Store (v0.3)

This document describes the on-disk content-addressable store (CAS) that
guroku uses to back its package cache. It is meant for contributors who
need to reason about how guroku stores tarballs, why it is structured the
way it is, and what guarantees the layout does and does not provide.

## What it is

The CAS lives at `~/.guroku/cas/`. Every cached package occupies a single
directory whose path is derived from the SHA-512 hash of the original
registry tarball:

```
~/.guroku/cas/<sha[0:2]>/<sha[2:]>/
```

The first two hex characters of the digest become the bucket directory,
and the remaining 126 characters become the entry directory. The entry
directory contains the extracted contents of the tarball plus a small
marker file (described below).

A concrete example for a tarball whose SHA-512 begins
`a3f1...`:

```
~/.guroku/cas/a3/f1d8c2...e9/
~/.guroku/cas/a3/f1d8c2...e9/package.json
~/.guroku/cas/a3/f1d8c2...e9/index.js
~/.guroku/cas/a3/f1d8c2...e9/.guroku-cas-ready
```

Because the path is derived purely from the hash of the bytes on the
wire, two registry records that ship byte-identical tarballs share a
single entry on disk. This is true regardless of the package name, the
version string, or which project pulled it in. The store is content
addressed, not name addressed.

## Why a content-addressable store

The motivation is plain deduplication. A typical developer machine ends
up with the same `react`, the same `lodash`, and the same
`@types/node` extracted dozens or hundreds of times across projects.
Storing each copy independently in every `node_modules` is wasteful both
on disk and on the install path (each copy is a fresh extract). A CAS
flips this: extract once per unique tarball, materialize many times via
links.

This aligns with how pnpm and bun structure their stores. Both maintain
a per-user content store and link from project `node_modules` into it.
guroku follows the same shape so that users moving between tools have a
predictable mental model.

## Why we hash the tarball, not file-by-file

pnpm v3 and later use a per-file CAS: a tarball is opened, each file
inside is hashed independently, and the store ends up keyed by file
hashes. The win is finer-grained dedup. If two versions of a package
differ in only one file, all the unchanged files are shared.

v0.3 deliberately does not do this. It hashes the whole tarball and
stores the extracted tree as a single unit. The reasons:

- The tarball hash is already in `dist.integrity` from the registry
  metadata, so we get it for free during the integrity check. No extra
  hashing pass is needed at install time.
- File-level dedup needs a more involved write path: stream the tarball,
  hash each entry, decide whether to write or skip, and rebuild the
  package layout from a manifest of file hashes. That is a meaningful
  amount of new code and new failure modes.
- Whole-tarball CAS already saves the dominant cost. Across versions of
  the same package on the same machine, and across copies of the same
  version installed in different projects, we avoid re-extracting the
  same bytes. The marginal win from per-file dedup matters more once we
  have heavy users with many parallel projects on small disks.

We expect to revisit per-file CAS in v0.4 or v0.5 once the v0.3 layout
has settled and we have real numbers on store sizes in the wild.

## Why the two-character prefix

Most filesystems handle large directories acceptably for reads but
slow down meaningfully for `readdir` and `stat` once a single
directory holds hundreds of thousands of entries. ext4 with `dir_index`,
APFS, and NTFS all degrade in different ways at scale, and shell tools
like `ls` and `find` get noticeably slow long before the filesystem
itself complains.

The two-hex-character prefix gives us 256 buckets. With a million
entries in the store, each bucket holds roughly 4000 subdirectories,
which is comfortable for every filesystem we care about. The cost is
trivial: one extra path component per entry. Going wider (three
characters, 4096 buckets) buys us nothing until the store is
implausibly large; going narrower defeats the point.

This is the same trick git uses for its loose object store
(`.git/objects/<sha[0:2]>/<sha[2:]>`).

## Atomicity

Extracting a tarball is not an atomic operation. We need to make sure
that a partially-extracted entry never appears to be a complete one,
even if the process is killed mid-extract or two processes race on the
same hash.

The mechanism lives in `store::ensure_extracted`. The protocol is:

1. Compute the target path `<bucket>/<rest>/`.
2. If the target already exists and `marker_present(target)` returns
   true, return immediately. The entry is complete.
3. Otherwise, extract the tarball into `<target>.tmp/` (a sibling
   directory).
4. After extraction succeeds, write `<target>.tmp/.guroku-cas-ready`.
5. Atomically rename `<target>.tmp` to `<target>`.

Step 5 is the key. On all supported platforms, `rename` of a directory
onto a non-existing target is atomic: either the directory is at the new
path or it is not. There is no intermediate state visible to other
processes.

If two processes race on the same hash, both will reach step 5. One
wins; its `rename` succeeds. The other one's `rename` fails because the
target now exists. The loser handles this by checking
`target.exists()` and removing its own `.tmp` directory. Both processes
then see a complete, valid entry at `<target>` and proceed.

If a process is killed between step 3 and step 5, no marker exists and
the partially-populated `<target>.tmp` directory is orphaned. A future
extract for the same hash will create a fresh `.tmp` (we do not reuse
the orphan) and the orphan can be cleaned up by garbage collection
later. The important property is that no other process ever observes
the half-populated tree as if it were complete, because the half-populated
tree never lives at the canonical path.

## The marker file

`.guroku-cas-ready` is a zero-byte file written into the entry just
before the rename. Its presence is a positive assertion that extraction
finished successfully and the tree is intact.

`store::marker_present` is what every reader checks before treating an
entry as usable. The check is a simple `target.join(".guroku-cas-ready").exists()`.

```
fn marker_present(entry: &Path) -> bool {
    entry.join(".guroku-cas-ready").exists()
}
```

Why we need it, given that the rename is atomic: in principle, the
atomic rename alone is enough to guarantee that the canonical path
either holds a complete entry or does not exist. In practice, the
marker is a defense-in-depth check against several real cases:

- A previous version of guroku, or a buggy build, that did not use the
  rename protocol and wrote directly to the canonical path.
- A user manually unpacking a tarball into the CAS path (we have seen
  this in bug reports).
- Filesystem corruption or aggressive `cp -r` recovery that produces a
  tree at the canonical path without the marker.

Without the marker, any of these would cause future reads to mistake a
half-populated directory for a complete entry and silently install
broken packages. With the marker, the worst case is a re-extract.

## Garbage collection

There is no garbage collector in v0.3. The store grows unboundedly. An
entry written once stays on disk until the user removes it manually or
deletes `~/.guroku/cas` wholesale. This is a deliberate v0.3 limitation
and is the most common piece of feedback we expect.

The v0.4 plan introduces:

- `guroku store gc` — walk every project lockfile reachable from the
  user's configured roots, compute the set of hashes in use, and remove
  CAS entries that are not referenced by any of them. Orphaned `.tmp`
  directories are removed unconditionally.
- `guroku store prune --age 30d` — remove CAS entries whose mtime is
  older than the given age, regardless of whether anything currently
  references them. Useful for users who do not maintain a stable set of
  project roots.

Until those land, the recommended workaround is `rm -rf ~/.guroku/cas`.
Subsequent installs will repopulate the store from the network, so the
only cost is bandwidth and time.

## Cross-filesystem concerns

The CAS itself does not care which filesystem it lives on. It only ever
extracts into and renames within `~/.guroku`, so the rename in step 5 of
the extraction protocol is always intra-filesystem and therefore atomic.

The cross-filesystem concern enters the picture in the linker, not the
store. When a project's `node_modules` lives on a different filesystem
than `~/.guroku` (which is common with mounted code volumes, dev
containers, and some CI setups), hardlinks from the project into the
store fail with `EXDEV`. The linker handles this with a copy fallback,
described in `storage.md`. The CAS layer is unaffected; it produces a
canonical extracted tree at a stable path, and how that tree gets
materialized into a project is the linker's problem.

## Threat model

guroku v0.3 trusts the filesystem. A sufficiently advanced attacker who
can write into `~/.guroku/cas` can:

- Place a forged tree at any hash path. Since we check the marker file
  but do not re-verify the hash on every read, a forged entry whose
  marker is present will be served as if it were the original tarball's
  contents.
- Replace files inside an existing entry. Same caveat: we do not
  re-hash the tree on every install.

We rely on standard filesystem permissions for protection. `~/.guroku`
is created mode `0700` so that other users on a multi-user machine
cannot write to it. We do not currently sign entries or re-verify them
on read.

A signed CAS is on the v0.5 roadmap. The plan is to record the expected
SHA-512 in a metadata file alongside `.guroku-cas-ready`, optionally
verify it on read (gated behind a flag, since re-hashing is expensive),
and eventually accept signatures from the registry.

For now: do not run guroku as root, do not share `~/.guroku` between
users, and treat the CAS the same way you would treat
`~/.cargo/registry` or `~/.npm/_cacache`.

## Diagnostics

A few one-liners are useful when investigating store behavior.

Total size of the store:

```
du -sh ~/.guroku/cas
```

Number of completed entries (count of marker files):

```
find ~/.guroku/cas -name '.guroku-cas-ready' | wc -l
```

Largest entries, in case a single dependency is dominating:

```
du -sh ~/.guroku/cas/*/* | sort -h | tail -n 20
```

Orphaned `.tmp` directories from crashed extracts:

```
find ~/.guroku/cas -maxdepth 2 -type d -name '*.tmp'
```

Buckets and how many entries each holds (sanity check on prefix
distribution):

```
for d in ~/.guroku/cas/*/; do printf '%s %d\n' "$d" "$(ls "$d" | wc -l)"; done | sort -k2 -n | tail
```

If a specific package looks broken, the fastest path is usually to find
its hash in the project lockfile, locate the entry under
`~/.guroku/cas/<sha[0:2]>/<sha[2:]>/`, check that
`.guroku-cas-ready` is present, and compare the contents against a
fresh extract of the registry tarball. Removing the entry directory
forces guroku to re-fetch and re-extract on the next install.
