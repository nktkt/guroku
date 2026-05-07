# Concurrent Installs

This document describes how guroku stays correct when multiple install
operations touch the same content-addressable store (CAS) at the same time.
It covers the race scenarios we expect, the invariants that keep them safe,
the things we deliberately do not do (file locks), and what can still go
wrong at the edges.

## 1. The scenarios

There are two flavours of concurrency that the install pipeline has to
tolerate.

### In-process

The fetch stage uses `buffer_unordered(8)` to drive up to eight CAS fetches
concurrently. In principle, two of these tasks can target the same content
hash if the resolver picked the same package version twice in the dep graph.
This is rare in practice -- the graph builder dedupes by `(name, version)`
before we ever reach the fetch stage -- but it is not impossible. A
`workspace.dependencies` table that pulls a package in two ways, plus an
overlap with a transitive dep, can produce two resolution nodes that share
a hash.

### Cross-process

The more common race is two processes:

- Two terminals running `guroku install` in two different projects that
  share a transitive package.
- A CI runner doing parallel jobs that all share `~/.guroku` (e.g. a matrix
  build, or `cargo nextest`-style sharding around install steps).
- A developer running `guroku install` while a background watcher (LSP,
  bundler, etc.) re-runs install on file changes.

In all of these, the OS is happily scheduling two CAS writes against the
same `<hash>` directory under `~/.guroku/cas/`.

## 2. The CAS race

The relevant code is `store::ensure_extracted_at`. The shape is:

```
target          = ~/.guroku/cas/<hash>
target_tmp      = ~/.guroku/cas/<hash>.tmp        (process-unique suffix)
marker_in_tmp   = <target_tmp>/.guroku-cas-ready
```

The sequence is:

1. If `marker_present(target)` -> return early. This is the happy path
   for already-extracted entries.
2. Otherwise, extract the tarball into `target_tmp`.
3. Write `.guroku-cas-ready` inside `target_tmp` (an empty marker file).
4. `fs::rename(target_tmp, target)`.

POSIX `rename(2)` is atomic for replacing one path with another on the
same filesystem. In our case the *destination does not yet exist* in the
common path -- both racers attempted extraction because neither saw the
marker. Both finish their tmp directory, both try to rename onto `target`.

What happens:

- Whichever rename hits the kernel first wins. `target` now exists and
  contains a complete, marker-stamped tree.
- The loser's `rename` call returns success on Linux/macOS (rename onto
  an existing directory is allowed if the target is empty, and otherwise
  is a permission/`ENOTEMPTY` error depending on the kernel). guroku
  handles both: after rename, the loser re-checks `target.exists() &&
  marker_present(target)`. If yes, the winner's tree is fine, and the
  loser deletes its own `target_tmp`.
- If the rename succeeded but somehow we are now the only marker-bearing
  tree -- i.e. we *were* the winner -- we return.

The outcome is that exactly one extracted tree ends up at `target`, with
its marker file intact, regardless of how many racers entered the
function.

## 3. Why we don't use file locks

The obvious alternative is `flock(2)` (or `fcntl` advisory locks) on a
per-hash lockfile. We do not do this.

- `flock` is unreliable on NFS. Some servers honour it, many do not, and
  the failure is silent: `flock` returns success but locks are not
  enforced across hosts.
- `flock` on FUSE filesystems (sshfs, rclone mount, etc.) has historically
  been broken or no-op.
- exFAT and other non-POSIX filesystems do not implement advisory locks.
- Windows has its own locking story, which we would have to handle
  separately anyway.

Atomic rename is the lowest common denominator that works on every
filesystem we are willing to support. It also avoids the lock-leak
problem: a crashed process holding a lockfile would block every future
install until manual cleanup; a crashed process mid-extraction just
leaves a `.tmp` directory, which is harmless and self-cleaning.

## 4. The marker file

`.guroku-cas-ready` exists for one reason: to distinguish a complete
extraction from an interrupted one.

Without the marker, the failure mode is:

1. Process A starts extracting into `target` directly (no tmp).
2. Process A is killed mid-extraction.
3. `target` now exists and is half-populated.
4. Process B looks up `target`, sees it exists, short-circuits, and
   uses a corrupt entry.

With the marker:

1. Extraction always happens in `target_tmp`.
2. The marker is the *last* thing written before rename.
3. The short-circuit gate is `marker_present(target)`, not
   `target.exists()`.
4. Therefore any path with a marker is, by construction, a complete
   extraction.

`marker_present` is a single `metadata().is_file()` check on the marker
path. It is the cheapest gate we can put before the short-circuit, and
it is the *only* signal trusted by the rest of the codebase to mean
"this entry is fully written; trust it".

## 5. Lockfile races

`guroku.lock` -- the per-project lockfile -- is written via `fs::write`,
which on POSIX is `open(O_WRONLY|O_CREAT|O_TRUNC) + write + close`. It
is not atomic against concurrent writers in the same project directory.

If two `guroku install` invocations race in the *same* project:

- Both resolve the dep graph (possibly to the same result, possibly not
  if the registry changed in between).
- Both write `guroku.lock`. Last writer wins. The file may briefly be
  truncated and then re-extended; a third reader at exactly that moment
  sees a short read.

We do not try to fix this. The advice is the same advice npm and pnpm
give: do not run two installs in the same project at the same time. The
project directory is the user's; the CAS is shared infrastructure.
Concurrency safety is a CAS-level concern, not a project-level one.

## 6. What can still go wrong

A few residual edge cases:

- **Windows directory rename.** On Windows, `MoveFileEx` of a directory
  whose target already exists is not atomic and may fail outright. In
  that case the loser sees `Err` from `fs::rename`, re-checks
  `marker_present(target)`, finds the winner's tree, and cleans up its
  own tmp. The end state is the same; it just takes the error path
  instead of the success-but-no-op path.
- **Out of disk during extraction.** The tmp directory is left behind.
  The next install run sees the tmp (it has a process-id suffix, so it
  will not collide with a fresh extraction) and removes it during the
  cleanup pass. No corruption: the marker was never written.
- **SIGKILL during rename.** The kernel either committed the rename or
  it did not; there is no in-between. So we end up with either no entry
  at `target` (next run re-extracts) or a complete entry at `target`
  (next run short-circuits). The tmp dir, if rename was midway, is
  cleaned up by the same cleanup pass.
- **Different filesystems.** If `~/.guroku/cas/<hash>.tmp` and
  `~/.guroku/cas/<hash>` are on different filesystems, `rename` returns
  `EXDEV`. We do not currently handle this; it would require a
  copy+fsync+unlink fallback. In practice the entire CAS is one
  directory tree on one filesystem, so this has not come up.

## 7. Diagnostics

A couple of one-liners are useful when debugging.

Show leftover in-flight (or crashed) extractions:

```sh
find ~/.guroku/cas -type d -name '*.tmp'
```

Count fully-written CAS entries:

```sh
find ~/.guroku/cas -type f -name '.guroku-cas-ready' | wc -l
```

Show CAS entries that exist but are missing their marker (these are the
"corrupt or pre-marker" entries; should be zero on a healthy store):

```sh
find ~/.guroku/cas -mindepth 1 -maxdepth 1 -type d \
  ! -name '*.tmp' \
  -exec test ! -f '{}/.guroku-cas-ready' \; \
  -print
```

Total CAS size:

```sh
du -sh ~/.guroku/cas
```

## 8. Future work

- `guroku store doctor`. A subcommand to walk `~/.guroku/cas`, find
  orphaned `.tmp` directories, find marker-less entries, and either
  report or remove them. Today this happens implicitly during install;
  we want an explicit command for users who want to audit their store.
- **Optional advisory lock on `~/.guroku`.** Not for correctness -- the
  rename pattern handles correctness -- but to bound resource use. A
  CI runner with 32 parallel jobs all extracting the same large
  package wastes 31 extractions worth of CPU and disk. A coarse
  advisory lock per hash, with a short timeout, would let only one
  extractor run while the others wait and short-circuit when the
  marker appears. This would be a pure optimisation; the semantics
  would not change.
- **`renameat2(RENAME_NOREPLACE)`** on Linux to fail fast on the loser
  side without having to re-stat the target. This is a small win and
  is Linux-specific, so it would have to be cfg-gated.
