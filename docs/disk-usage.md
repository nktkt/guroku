# Disk Usage

This document explains how to measure and reason about the disk space
guroku consumes on your machine. It is the operational counterpart to
`docs/storage.md`, which describes *what* lives on disk and *where*;
this document is about *how big* those things are and *what to do*
when they grow.

If you are looking for the on-disk layout (CAS path scheme, project
`node_modules/.guroku/` structure, lockfile location), read
`docs/storage.md` first and come back here.

---

## 1. Why this matters

npm uses a flat `node_modules` model: every project gets its own
private copy of every dependency, fully extracted. If ten projects on
your laptop depend on `react`, you have ten copies of `react` on disk.
For a non-trivial dependency graph this is the dominant disk cost on
a developer machine.

Strict-layout package managers (guroku v0.3+ and pnpm) take a
different approach. Package contents are stored exactly once in a
content-addressable store (CAS), and each project's `node_modules`
references those bytes via hardlinks. The savings scale with how many
projects share the same dependency versions.

The headline consequence:

- **Few projects, all using different dep versions**: savings are
  small. Each unique version still costs its own bytes.
- **Many projects sharing a common stack** (React, TypeScript, a
  bundler, a test runner, lint tooling): savings are large. The
  shared bytes are paid for once, not N times.

The rest of this document is about understanding *where* those bytes
live and *how* to look at them with standard tools.

---

## 2. Where disk is actually used

guroku uses three on-disk locations. Only one of them is large.

### `~/.guroku/cas/`

The content-addressable store. This is where the real bytes live:
extracted package contents, keyed by tarball SHA-256. Every package
version you have ever installed across every project on your machine
is here, exactly once. This is by far the largest of the three.

Inside the CAS, each entry is a directory tree mirroring the package
contents, plus a `.guroku-cas-ready` marker file written atomically
when extraction finishes. The marker is what guroku looks for when
deciding whether a CAS entry is reusable.

### `~/.guroku/cache/metadata/`

Cached registry metadata: package documents, version manifests,
tarball URLs. These are small JSON files. Even on a busy machine
this directory is usually well under 100 MB.

### `<project>/node_modules/`

Per-project. This *looks* large in `du` output but is mostly
references to the CAS, not real bytes:

- Package files are **hardlinks** into `~/.guroku/cas/`. A hardlink
  shares an inode with its target, so the file content is paid for
  once globally, not once per link.
- Module entry points (the things `require()` resolves to) are
  **symlinks** into `node_modules/.guroku/`. Symlinks are tiny — a
  few bytes each plus an inode.
- The `node_modules/.guroku/` virtual store is itself directories
  full of symlinks. Directory entries take a small amount of space
  per entry but no per-file content cost.

Net: the *real* disk usage of a project's `node_modules` after the
CAS already has its dependencies is dominated by directory entries
and symlinks, not file content.

---

## 3. Measuring

All commands below use standard POSIX-ish tools. On macOS, `du -x`
behaves slightly differently than on Linux; see notes inline.

```sh
# Total CAS size (the big one):
du -sh ~/.guroku/cas

# How many CAS entries exist (count of completed marker files):
find ~/.guroku/cas -name '.guroku-cas-ready' | wc -l

# Metadata cache size (should be small):
du -sh ~/.guroku/cache/metadata

# Apparent size of a project's node_modules
# (counts every hardlink, so this is the "if it weren't deduped" number):
du -sh node_modules

# Real disk cost (hardlinks counted once):
du -shx --apparent-size node_modules     # apparent (Linux/GNU coreutils)
du -shx node_modules                     # real (Linux/GNU coreutils)
```

On macOS, `du -shx node_modules` reports the real, dedup-aware
number by default; `--apparent-size` is a GNU coreutils flag and is
not available in BSD `du`. To get the apparent number on macOS,
install GNU coreutils (`brew install coreutils`) and use `gdu`.

A useful one-liner for "how many distinct package versions are on
this machine":

```sh
find ~/.guroku/cas -mindepth 2 -maxdepth 2 -type d | wc -l
```

The exact depth depends on the CAS sharding scheme — adjust if
`docs/storage.md` documents a different layout.

---

## 4. Why the two `du` numbers differ

When you run both `du -sh` and `du -shx` on the same `node_modules`,
the apparent number is usually much larger than the real number.
That is not a bug. It is the entire point of the CAS.

- `du` without dedup (`--apparent-size` on Linux, default on
  macOS-without-`-x` on some filesystems) walks the directory tree
  and adds up the size of every file it sees. It does not notice
  that ten files in different directories all point at the same
  inode. It counts the bytes ten times.
- `du` with dedup (`-x` on macOS, default on GNU `du` for hardlinks
  encountered more than once during a single invocation) tracks
  inodes and only charges each one once.

The difference between the two is, by definition, the bytes you
saved by hardlinking instead of copying. If `du -sh` says
`node_modules` is 800 MB and `du -shx` says it is 40 MB, the CAS
saved you 760 MB on this project alone — and those 40 MB of
"real" cost is mostly directory entries and symlinks, not content.

---

## 5. Across-project savings example

Three projects on the same machine, each with `lodash@4.17.21` in
its dependency tree.

Without a CAS (npm-style flat layout):

| Project   | lodash bytes |
| --------- | ------------ |
| project-a | ~1.4 MB      |
| project-b | ~1.4 MB      |
| project-c | ~1.4 MB      |
| **Total** | **~4.2 MB**  |

With guroku's CAS:

| Location         | lodash bytes                             |
| ---------------- | ---------------------------------------- |
| `~/.guroku/cas/` | ~1.4 MB (one set of bytes)               |
| project-a        | ~0 (hardlinks + symlinks)                |
| project-b        | ~0 (hardlinks + symlinks)                |
| project-c        | ~0 (hardlinks + symlinks)                |
| **Total**        | **~1.4 MB**                              |

Lodash is a small example. Substitute `typescript`, `next`, or
`@aws-sdk/*` and the savings get correspondingly larger.

---

## 6. Across-version savings limits

The CAS deduplicates by tarball SHA-256. Two different versions of
the same package have different tarballs, different SHAs, and
therefore different CAS entries. There is no sharing between them.

Concretely: if you have `huge-package@1.2.3` (100 MB extracted) in
one project and bump another project to `huge-package@1.2.4`
(also 100 MB extracted, with maybe one file changed), guroku
stores both. The CAS will hold ~200 MB for those two versions, the
same as npm would.

This is a real cost. It hits hardest in monorepos with mixed
versions and in machines that have accumulated old projects pinned
to old major versions.

A planned future milestone introduces **per-file CAS** (similar to
pnpm), which deduplicates at the file level rather than the tarball
level. Two patch-version bumps of a 100 MB package that share 99 MB
of identical files would then cost ~101 MB instead of ~200 MB.
guroku v0.3 is per-tarball; per-file is on the roadmap.

---

## 7. What happens when the CAS grows too big

### Today (v0.3)

Nothing automatic. The CAS grows monotonically. Every package
version you have ever installed stays in `~/.guroku/cas/` until you
explicitly delete it, regardless of whether any project on your
machine still references it.

### Workaround

The CAS is a pure cache. Deleting it is safe in the sense that no
project's lockfile or source tree depends on it being present:

```sh
rm -rf ~/.guroku/cas
```

The next `guroku install` in any project will re-fetch the tarballs
it needs from the registry and repopulate the CAS. This is slow
(network-bound) but correct. You will *not* lose any project state.

### Future

A planned subcommand will provide proper garbage collection:

```sh
guroku store gc --age 30d
```

This will scan every lockfile under known project roots, build the
set of CAS entries those lockfiles still reference, and evict
everything else that has not been touched in the given window.
Until that ships, `rm -rf ~/.guroku/cas` is the only option.

---

## 8. Comparison with other managers

| Manager     | Per-project layout | Global CAS         | Hardlinks         | Symlink layout |
| ----------- | ------------------ | ------------------ | ----------------- | -------------- |
| npm         | flat               | no                 | no                | no             |
| pnpm        | strict             | yes                | yes (per-file)    | yes            |
| guroku v0.3 | strict             | yes (per-tarball)  | yes (per-tarball) | yes            |
| bun         | flat               | yes                | yes               | no             |

A few notes on this table:

- "Strict" layout means each package only sees the dependencies it
  actually declared. "Flat" layout hoists everything into one shared
  directory, which is how npm gets its accidental-dependency
  problem.
- "Per-file" CAS deduplicates at the granularity of individual files
  inside packages. "Per-tarball" deduplicates at the granularity of
  whole package versions. Per-file is strictly better for disk but
  more complex to implement and reason about.
- bun has a global CAS and uses hardlinks, but its `node_modules`
  layout is flat (npm-style), so it does not get the
  strict-resolution correctness benefits. This is a deliberate
  compatibility tradeoff on bun's part.

---

## 9. Inode considerations

Hardlinks share inodes with their targets, but every symlink and
every directory still costs its own inode. Strict-layout managers
(guroku, pnpm) create *many* symlinks and directories — one per
package per project, plus the `.guroku/` virtual store.

On most modern filesystems (APFS, ext4 with default settings,
Btrfs, XFS) inode counts are effectively unlimited and this is a
non-issue. On older filesystems or filesystems with statically
sized inode tables, very large monorepos can in principle exhaust
the inode budget before exhausting disk space.

For the detailed discussion — which filesystems are affected, how
to check inode usage, and what to do if you actually hit a
ceiling — see `docs/internals/hardlinks.md`.

---

## 10. FAQ

**Q: I deleted `node_modules` and disk usage barely went down. Why?**

Because `node_modules` was almost entirely hardlinks and symlinks,
not real content. The real content lives in `~/.guroku/cas/` and
is still there. If you want to reclaim that space too, delete the
CAS:

```sh
rm -rf ~/.guroku/cas
```

The next install in any project on the machine will re-fetch what
it needs.

**Q: Can I move `~/.guroku/` to a different drive?**

Not in v0.3. The path is hardcoded to `$HOME/.guroku`. A
configurable store path is planned but not currently shipped.

If you need this today, the workarounds are filesystem-level: a
symlink at `~/.guroku` pointing at the target drive, or a bind
mount. Both work because guroku does not validate the path; it
just opens it.

**Q: Why is my `node_modules` huge in `du`?**

Almost certainly because you ran `du -sh` (or plain `du`) without
the dedup flag. That counts every hardlink as if it were a separate
copy. Use `du -shx node_modules` (macOS, default dedup) or
`du -sh node_modules` with GNU `du` (default dedup on Linux for
hardlinks seen during the same walk) for the real number.

If the real number is also unexpectedly huge, the likely causes
are: a large number of distinct package versions in the project,
a few legitimately large packages (TypeScript, AWS SDK,
Playwright browsers if installed via postinstall), or a postinstall
script that wrote real files into `node_modules` rather than
linking. The CAS cannot deduplicate bytes that postinstall scripts
generate at install time.

**Q: Does the CAS get corrupted if I `rm -rf` it mid-install?**

It should not, because the `.guroku-cas-ready` marker is the
acceptance gate: a CAS entry without that marker is treated as
missing and re-extracted. A half-deleted entry will simply be
re-fetched on next install. If you see odd behavior after a
mid-operation kill, deleting the affected CAS subtree and
reinstalling is always safe.

**Q: How do I see which projects on my machine are using the CAS?**

There is no built-in command for this in v0.3. The planned
`guroku store gc` subcommand will need to discover project roots
to compute the live set, and the same discovery will be exposed
to users. Until then, you would have to grep your filesystem for
lockfiles yourself.

---

## See also

- `docs/storage.md` — on-disk layout reference.
- `docs/internals/hardlinks.md` — hardlink and inode details.
- `docs/cli-reference.md` — full CLI surface (note: `store gc` is
  not yet implemented).
