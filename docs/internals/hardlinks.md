# Hardlinks: the v0.3 linker

This document explains how guroku v0.3 materializes `node_modules` from
its content-addressed store (CAS) using POSIX hardlinks, why that gives
us free deduplication across projects, and where the approach has rough
edges.

If you have not read [`storage.md`](storage.md) and
[`integrity.md`](integrity.md), do so first; this doc assumes you know
what the CAS layout looks like and how a tarball arrives in
`~/.guroku/store/<sha512>/`.

## 1. What a hardlink is

A hardlink is a directory entry that points at the same on-disk inode
as another directory entry. Two paths, one underlying file. The bytes
are not duplicated; the kernel maintains a reference count on the
inode, and the file's data only goes away when the last directory entry
referencing it is removed.

```
$ echo hello > a.txt
$ ln a.txt b.txt          # hardlink, not a symlink
$ stat -f '%i %l' a.txt
123456789 2
$ stat -f '%i %l' b.txt
123456789 2
```

Both entries share inode `123456789`, and the link count is `2`.
`unlink`-ing one entry does not delete the data while another entry
still references it:

```
$ rm a.txt
$ cat b.txt
hello
$ stat -f '%i %l' b.txt
123456789 1
```

A few properties that matter for the linker:

- Equality of bytes is automatic: a hardlinked file *is* the same file.
  No copy, no drift, no checksum needed.
- Hardlinks live within a single filesystem. You cannot hardlink across
  mount points; the kernel returns `EXDEV`.
- Hardlinks point at *files*, not directories. (Most modern Unixes
  forbid directory hardlinks outright.)
- File metadata (mtime, mode, owner) is a property of the inode, not
  the directory entry. Modifying any link modifies all of them.

## 2. Why guroku v0.3 uses hardlinks

The CAS is content-addressed: a tarball's extracted tree lives at a
path keyed by its sha512. If two projects depend on, say,
`lodash@4.17.21`, they both resolve to the *same* extracted tree in the
store.

If we copied that tree into each project's `node_modules`, a 200 MB
shared dependency set across 20 projects would cost roughly 4 GB on
disk. Hardlinking instead means:

- The CAS holds one canonical copy of every file.
- Each project's `node_modules/<pkg>/` contains directory entries
  pointing at the CAS inodes.
- The kernel does the dedup for free; `du -sh ~/projects` reflects the
  *unique* bytes, not the cumulative ones.

Concretely: 200 MB shared across 20 projects costs roughly 200 MB on
disk, not 4 GB. The savings scale with the number of projects, not
linearly with their declared sizes.

This is the same insight that motivates pnpm's design (see section
10), but applied at a different granularity.

## 3. `link_hardlink_tree` walkthrough

The linker is a recursive walk over a CAS package directory. The
function signature is:

```rust
pub fn link_hardlink_tree(src: &Path, dst: &Path) -> Result<()>;
```

`src` is the CAS path (e.g. `~/.guroku/store/sha512-abc.../`) and
`dst` is the per-package directory inside `node_modules` (e.g.
`my-app/node_modules/.guroku/lodash@4.17.21/node_modules/lodash/`).

The walk:

```rust
for entry in fs::read_dir(src)? {
    let entry = entry?;
    let name = entry.file_name();

    if name == ".guroku-cas-ready" {
        continue; // see section 4
    }

    let src_path = entry.path();
    let dst_path = dst.join(&name);
    let ft = entry.file_type()?;

    if ft.is_dir() {
        fs::create_dir_all(&dst_path)?;
        link_hardlink_tree(&src_path, &dst_path)?;
    } else if ft.is_symlink() {
        let target = fs::read_link(&src_path)?;
        symlink(&target, &dst_path)?;
    } else {
        link_or_copy(&src_path, &dst_path)?;
    }
}
```

Three cases:

1. **Directory.** `mkdir -p` the destination, recurse.
2. **Symlink.** Read the link target out of the tarball-extracted tree
   and reproduce it at the destination. We do *not* follow the symlink
   and hardlink the target; doing so would silently change the
   semantics of relative symlinks inside the package.
3. **Regular file.** `fs::hard_link(src, dst)`, with a fallback
   described in section 5.

There is no special case for executables, no chmod step, no
post-processing. The CAS is already correct because the extractor
preserved tar metadata; the linker just exposes it under a new path.

## 4. The `.guroku-cas-ready` filter

When the extractor finishes writing a package into the CAS, it writes
a zero-byte sentinel file named `.guroku-cas-ready` *inside* the
package directory. The fetch path checks for this sentinel before
treating the directory as usable; if it is missing, we assume a
previous extraction crashed mid-write and re-do it.

This sentinel is a CAS implementation detail. It must not show up in
the user's `node_modules`, where it would be visible to tools that
walk the tree (bundlers, antivirus, IDE indexers).

The linker therefore filters it explicitly:

```rust
if name == ".guroku-cas-ready" {
    continue;
}
```

The check is at the top level of each `read_dir` iteration. It is not
recursive: the sentinel only ever appears at the root of a CAS package
directory, so a single name comparison suffices.

If the published package itself contained a file named
`.guroku-cas-ready`, we would shadow it. That is acceptable; the name
is reserved.

## 5. Hardlink fallback semantics

`fs::hard_link` can fail for reasons that are not the user's fault:

- The CAS and the project live on different filesystems (`EXDEV`).
- The destination filesystem is exFAT, FAT32, or another format that
  does not support hardlinks at all.
- The destination is on a SMB or NFS mount with hardlink support
  disabled.
- The destination volume is a snapshot or copy-on-write target where
  the kernel rejects new hardlinks.
- Windows NTFS junctions and certain reparse points reject
  cross-volume hardlink attempts.

The linker handles all of these uniformly:

```rust
fn link_or_copy(src: &Path, dst: &Path) -> Result<()> {
    match fs::hard_link(src, dst) {
        Ok(()) => Ok(()),
        Err(_) => {
            fs::copy(src, dst)?;
            Ok(())
        }
    }
}
```

The user gets correct files either way. What they lose, on the
fallback path, is dedup: the bytes are physically copied into the
project. For most users this is fine; the cases that hit the fallback
are usually small.

We deliberately do not distinguish between fallback reasons. A
classification would require platform-specific errno mapping and
would not change the response. The on-disk outcome is what matters.

## 6. What hardlinks cannot do

Two hard limits drive the rest of v0.3's design:

1. **Hardlinks do not cross filesystems.** If `~/.guroku` is on the
   user's home volume and the project lives on an external SSD, every
   file falls back to copy. We document this in
   [`storage.md`](storage.md) and recommend keeping the store on the
   same volume as your projects.
2. **Hardlinks point at files, never directories.** You cannot
   `ln /cas/lodash /node_modules/lodash` and have the directory be
   shared. Most Unixes forbid this outright; the few that ever
   permitted it (early HFS, certain Solaris configurations) made the
   filesystem fragile in exchange.

The second limit is why we use *symlinks*, not hardlinks, between the
`.guroku/` virtual store and the user-visible package paths. The
layout is:

```
node_modules/
  lodash -> .guroku/lodash@4.17.21/node_modules/lodash   (symlink)
  .guroku/
    lodash@4.17.21/
      node_modules/
        lodash/                                          (hardlinks here)
          package.json
          lodash.js
          ...
```

The symlink at `node_modules/lodash` is what Node's resolver follows.
The hardlinks under `.guroku/` are what dedup against the CAS. Two
mechanisms, one structure.

## 7. Mtime and chmod surprises

Hardlinks share an inode, which means they share metadata. If a user
edits `node_modules/lodash/lodash.js` in place:

- The write goes to the inode shared with the CAS copy.
- Every other project that hardlinked the same CAS file now sees the
  edited content.
- `git diff` in another project will suddenly show changes the user
  did not make there.

This is out-of-spec. Editing files inside `node_modules` has never
been a supported workflow; tools like patch-package exist precisely
because of this.

Future work could mitigate the foot-gun:

- On POSIX, mark CAS files read-only (`chmod a-w`). An attempted edit
  through a hardlink would then fail with `EACCES`, surfacing the
  problem at write time. The trade-off: build tools that touch
  `node_modules` for legitimate reasons (e.g. patching) would also
  fail, requiring a guroku-aware patch flow.
- Detect edits via integrity checks during the next install and warn.

For v0.3 we accept the sharp edge and rely on documentation.

## 8. Editor sniffing

Some editors do not write files in place. The classic pattern is:

1. Write the new content to a temp file in the same directory.
2. `rename(temp, target)`.

vim has done this for decades (`:set backupcopy=no`); some IDEs and
formatters do the same. The rename atomically replaces the directory
entry with one pointing at a *new* inode. The CAS inode is untouched;
the user's project now has its own private copy.

This is fine, and in fact is the failure mode we *want* if a user
insists on editing under `node_modules`:

- The user's edit lands on a fresh file.
- Other projects sharing the original CAS inode are unaffected.
- The CAS itself is unaffected; integrity checks still pass.

The cost is a small loss of dedup for that one file in that one
project, which is negligible.

The takeaway: rename-style editors degrade gracefully. In-place
editors (the default `dd`/`>>` shell idioms, some build tools) do not.

## 9. Inode pressure

A pathological JavaScript project can have hundreds of thousands of
small files. Each hardlink consumes a directory entry, which is cheap,
but also nominally a reference on an inode, which depending on the
filesystem may be tracked in a fixed-size table.

Modern filesystems handle this without complaint:

- **APFS** (macOS): inodes are 64-bit, allocated on demand, no
  practical limit.
- **ext4** (Linux): inode count is fixed at format time but typically
  generous; `mkfs.ext4` defaults to one inode per ~16 KB of disk,
  which on a 500 GB volume is ~30M inodes. Plenty.
- **btrfs** (Linux), **ZFS**: dynamic, no practical limit.
- **NTFS** (Windows): MFT entries are dynamic.

Older or constrained filesystems do not:

- **FAT32, exFAT**: no hardlink support at all. The fallback copy path
  handles these.
- **ext2/3 with a tight inode-to-block ratio**: can run out of inodes
  before running out of space, especially on volumes formatted for
  large media files.
- Some embedded and network filesystems impose their own limits.

The fallback copy path covers the no-hardlink cases. Inode-exhaustion
on a hardlink-supporting filesystem is rare enough that we do not
detect it specifically; the user will see an `ENOSPC`-class error from
`fs::hard_link`, fall back to `fs::copy`, and likely hit the same
error there. That is the correct outcome: we cannot install into a
filesystem that is full.

## 10. Comparison with pnpm

pnpm pioneered this design space. Its CAS is keyed per-file: every
unique file across every package version sits at one path under
`~/.local/share/pnpm/store/`, and pnpm hardlinks each file
individually into a per-package virtual-store directory.

guroku v0.3's CAS is keyed per-tarball: the entire extracted contents
of one tarball live under one CAS path, and the linker hardlinks the
files of *that tarball's tree* into a per-package directory.

Practical differences:

| Aspect                 | pnpm (per-file CAS) | guroku v0.3 (per-tarball CAS) |
|------------------------|---------------------|-------------------------------|
| Dedup granularity      | File                | Tarball                       |
| Store layout           | Flat by content hash| Flat by tarball hash          |
| Identical file in two different versions | Shared | Not shared       |
| Verification cost      | Per-file hash       | Per-tarball hash              |
| Linker walk            | Manifest-driven     | Filesystem walk               |

For typical npm dependency graphs the outcomes are very close. Most
files inside a published tarball are unique to that tarball; the
overlap pnpm captures and we miss is mostly README files, license
boilerplate, and the occasional re-exported helper. Real measurements
on representative projects show under five percent extra disk usage
for guroku relative to pnpm.

The win, for us, is that the CAS layout matches the integrity model:
one tarball, one hash, one directory, one verification. It is a
straightforward extension of how the resolver thinks about packages.

## See also

- [`storage.md`](storage.md): on-disk layout of the CAS.
- [`integrity.md`](integrity.md): how tarball hashes are verified.
- [`dependency-graph.md`](dependency-graph.md): how the symlink layer
  on top of the hardlinks is built.
