# hardlink-fallback

A README-only example documenting what happens when hardlinks fail and
guroku's linker falls back to a plain copy.

## What this example shows

guroku's default linker uses hardlinks from the global content-addressed
store (`~/.guroku/cas/`) into each project's `node_modules/`. Hardlinks are
cheap, deduped, and instant -- but only when both ends live on the same
filesystem.

When a hardlink can't be created (most often: cross-filesystem), guroku
silently falls back to copying the file. The user gets correct, working
files; the only thing lost is on-disk deduplication.

This example documents the fallback path so you know what to expect, how
to detect it, and how to avoid it when disk space matters.

## The trigger

The fallback kicks in whenever `fs::hard_link(src, dst)` returns an error.
The most common cause is:

- `~/.guroku/cas/` lives on filesystem **X**
- `<project>/node_modules/` lives on filesystem **Y**

POSIX does not allow hardlinks across filesystems, and neither does
macOS, Linux, or Windows. The kernel rejects the syscall with `EXDEV`
(`Invalid cross-device link`).

Other (rarer) triggers:

- The destination filesystem doesn't support hardlinks (e.g. some FUSE
  mounts, certain network filesystems).
- Permission denied on the source inode.
- Hardlink count limit reached on the source inode.

## What guroku does

In the linker (`link_hardlink_tree`), each file is materialized like
this:

```
fs::hard_link(src, dst)        // try the cheap path first
    .or_else(|_| fs::copy(src, dst).map(|_| ()))?;  // fall back to copy
```

If `hard_link` fails for any reason, guroku retries the same `(src, dst)`
pair with `fs::copy`. The user sees the file appear in `node_modules/`
either way. No error is surfaced, and -- currently -- no log line is
emitted (see "Open issue" below).

## How to reproduce

### macOS

1. Install guroku on your internal SSD (default location:
   `~/.guroku/`, which lives on `/`).
2. Plug in an external drive formatted as exFAT, APFS, or HFS+. It will
   mount under `/Volumes/EXTERNAL`.
3. Copy or create a project on the external drive:

   ```sh
   cp -r ./myproj /Volumes/EXTERNAL/myproj
   guroku install --cwd /Volumes/EXTERNAL/myproj
   ```

4. The install will succeed. Every file inside
   `/Volumes/EXTERNAL/myproj/node_modules/` will be a real copy, not a
   hardlink.

### Linux

Same idea, with a different mountpoint:

```sh
sudo mount -t tmpfs -o size=2G tmpfs /mnt/scratch
cp -r ./myproj /mnt/scratch/myproj
guroku install --cwd /mnt/scratch/myproj
```

`~/.guroku/cas/` lives on your root filesystem; `/mnt/scratch` is a
separate tmpfs. The hardlink call returns `EXDEV` and guroku falls back
to copy.

## Diagnose

`GUROKU_LOG=debug guroku install` won't currently show the fallback
explicitly -- there is no log line for it yet. For now, you can detect
it after the fact by comparing inode numbers:

```sh
# Same inode number = hardlink. Different = copy.
ls -li ~/.guroku/cas/<some-hash>/<some-file>
ls -li node_modules/.guroku/<...>/node_modules/<pkg>/<some-file>
```

The first column of `ls -li` is the inode number. If the two numbers
match, the file is a hardlink and you got the dedup. If they differ, the
fallback fired and you have a real copy.

You can also compare on-disk size:

```sh
du -sh ~/.guroku/cas/
du -sh node_modules/
```

If `node_modules/` is roughly the same size as the CAS share for this
project, hardlinks worked. If it's significantly larger, you're paying
the copy cost.

## Disk implications

When the fallback fires, **each project pays the package's bytes on
disk**. The CAS still holds one canonical copy in `~/.guroku/cas/`, but
the per-project `node_modules/` is no longer free.

A typical Node.js project's `node_modules/` is 200 MB - 1 GB. With
hardlinks, that's effectively zero marginal disk on top of the CAS.
With the copy fallback, it's the full size, every time, per project.

If you have ten projects on an external drive, that's 2 - 10 GB of
duplicated bytes the linker would normally have eliminated.

## Open issue: log fallback path

Currently the fallback is completely silent. There is no warning, no
debug log, and no metric. From the user's perspective, install just
works -- they only notice the disk usage afterwards.

This is tracked as future work. A debug-level log line on every fallback
(or a single warn-level summary at the end of install) would make the
behavior discoverable. PR welcome.

## Workarounds

The simplest fix is to keep `~/.guroku/` and your projects on the same
filesystem. Concretely:

- Don't put projects on external drives if you care about dedup.
- Or, set `GUROKU_HOME` to a directory on the same filesystem as the
  project:

  ```sh
  GUROKU_HOME=/Volumes/EXTERNAL/.guroku guroku install --cwd /Volumes/EXTERNAL/myproj
  ```

  This trades one global CAS for one CAS per filesystem, which is still
  cheaper than copying per project.

## Related docs

- `docs/internals/hardlinks.md` -- linker design, CAS layout, and the
  full lifecycle of a materialized file.
