# Storage reference

This document describes where guroku puts files on your machine, what
each location is for, and how to safely inspect, clean, and back up
guroku's on-disk state.

This is a user-facing reference. If you are looking for the contributor
deep-dive on how the content-addressable store works internally
(extraction algorithm, locking, integrity checks), see
`docs/internals/cas.md` instead.

---

## 1. Where guroku puts things

guroku stores all of its global state under `~/.guroku`. Project-local
state lives in your project's `node_modules` directory, exactly like
npm and pnpm.

```
~/.guroku/
├── cas/                               # extracted package contents (sha512-keyed)
│   ├── ab/<long-hex-rest>/...
│   └── ...
├── cache/
│   └── metadata/                      # ETag-aware registry response cache
│       ├── lodash.json
│       └── lodash.etag
└── store/                             # legacy (v0.1/v0.2) — unused in v0.3
```

Inside a project:

```
<your-project>/
├── package.json
├── guroku.lock
└── node_modules/
    ├── .guroku/
    │   ├── lodash@4.17.21/
    │   │   └── node_modules/
    │   │       └── lodash/...         # files hardlinked into ~/.guroku/cas
    │   └── chalk@5.3.0/
    │       └── node_modules/
    │           └── chalk/...
    ├── lodash -> .guroku/lodash@4.17.21/node_modules/lodash
    └── chalk  -> .guroku/chalk@5.3.0/node_modules/chalk
```

The `~/.guroku/store/` directory is left over from guroku v0.1 and
v0.2. It is unused in v0.3. You can delete it; guroku will not
recreate it.

---

## 2. What's in `~/.guroku/cas`

`~/.guroku/cas` is guroku's content-addressable store (CAS). It holds
the extracted contents of every package version your machine has ever
installed, across every project.

- The store is keyed by the SHA-512 of each tarball. The first two
  hex characters become the top-level shard directory, and the rest
  of the hash names the directory underneath.
- Each unique tarball is stored exactly once. If five projects on
  your machine all depend on `lodash@4.17.21`, the bytes live in the
  CAS once, not five times.
- Files inside a project's package directories (under
  `node_modules/.guroku/<name>@<version>/...`) are *hardlinks* into
  the CAS, not copies. They share an inode with the canonical CAS
  file.

The result: install a package once, pay the disk cost once, no matter
how many projects use it.

---

## 3. What's in `~/.guroku/cache/metadata`

This directory caches HTTP responses from the npm registry. For each
package name guroku has ever fetched metadata for, it stores two
files:

- `<name>.json` — the registry response body (the package manifest
  / packument).
- `<name>.etag` — the `ETag` header that came with that response.

On the next install, guroku sends an `If-None-Match: <etag>` header.
If the package hasn't changed, the registry replies `304 Not Modified`
and guroku reuses the cached body. This makes repeat installs much
faster and lets you re-run `guroku install` offline as long as nothing
in your dep tree has actually changed.

This cache is purely an optimization. It is safe to delete at any
time; guroku will refetch what it needs.

---

## 4. What's in your project's `node_modules`

guroku produces a strict pnpm-style `node_modules` layout. Two key
properties:

1. Every package goes through `node_modules/.guroku/<name>@<version>/`
   first. That subtree is the only place a package physically lives.
2. Direct dependencies of your project get a *symlink* at
   `node_modules/<name>` pointing into `.guroku/`.

Concretely, for a project that depends on `lodash` and `chalk`:

```
node_modules/
├── .guroku/
│   ├── lodash@4.17.21/
│   │   └── node_modules/
│   │       └── lodash/
│   │           ├── package.json
│   │           └── ...                # hardlinks into ~/.guroku/cas
│   └── chalk@5.3.0/
│       └── node_modules/
│           └── chalk/...
├── lodash -> .guroku/lodash@4.17.21/node_modules/lodash
└── chalk  -> .guroku/chalk@5.3.0/node_modules/chalk
```

Why this layout?

- Node's module resolution algorithm walks up `node_modules`
  directories. By placing each package under its own
  `<name>@<version>/node_modules/<name>/` directory, guroku ensures a
  package can only `require` what it has actually declared as a
  dependency. No accidental hoisted access.
- The surface symlinks at `node_modules/<name>` keep the project root
  looking familiar to tools that scan `node_modules`.

The files inside each package directory are hardlinked back to
`~/.guroku/cas`. They are real files on the filesystem (not symlinks)
but they share an inode with the CAS copy.

---

## 5. Disk usage

What it costs you:

- **First-ever install of a project.** Cost is the size of every
  unique tarball, extracted, written into `~/.guroku/cas`.
- **Second project that shares the same dependencies.** Cost is just
  the symlinks and hardlinks for that project's `node_modules` —
  typically tens of kilobytes total. The bytes are already in the CAS
  and are not duplicated.
- **A project with some shared and some new dependencies.** Cost is
  the size of the new tarballs only.

Inspect what's on disk:

```sh
# How big is the global CAS?
du -sh ~/.guroku/cas

# How big is the registry metadata cache?
du -sh ~/.guroku/cache/metadata

# How big does this project's node_modules look on disk?
du -sh node_modules

# Note: du counts hardlinked files once per inode it sees, so
# `du -sh node_modules` will report the *apparent* size of the deps.
# That space is mostly shared with ~/.guroku/cas, not duplicated.
```

If `du -sh ~/.guroku/cas` looks larger than you expect, remember it
accumulates every package version you have *ever* installed. There is
no garbage collector yet (see section 6).

---

## 6. Cleaning up

### Drop a single project's `node_modules`

```sh
rm -rf node_modules
```

This is always safe. Re-run `guroku install` to rebuild it. Bytes are
not refetched from the network — they're rehydrated from the CAS via
hardlink.

### Drop the registry metadata cache

```sh
rm -rf ~/.guroku/cache/metadata
```

This forces the next install to refetch package manifests from the
registry. Use this if you suspect the cache is corrupt or stale.

### Drop the entire CAS

```sh
rm -rf ~/.guroku/cas
```

This frees disk space. The next install in any project will refetch
the tarballs it needs from the registry and repopulate the CAS.

### What about a `store gc` command?

guroku does **not** yet ship a `store gc` command that can prune
unreferenced versions from the CAS while leaving still-in-use
versions in place. This is tracked for v0.4. For now, the all-or-
nothing `rm -rf ~/.guroku/cas` is your only built-in option.

---

## 7. Don't edit files in `node_modules/<name>/...`

Files inside your project's `node_modules` are **hardlinks** into
`~/.guroku/cas`. Editing one of those files modifies the CAS copy,
which is shared across every project on your machine that uses that
package version.

Concretely, if you do this:

```sh
# DON'T DO THIS
vim node_modules/lodash/lodash.js
```

…you have just modified the canonical copy of `lodash@4.17.21` in
`~/.guroku/cas`. Every other project on your machine that depends on
`lodash@4.17.21` will see your edits. This is almost never what you
want, and it leaves `~/.guroku/cas` in a state where its on-disk
contents no longer match the SHA-512 hash that names the directory.

If you want to experiment with patching a dependency, copy it out
first:

```sh
cp -R node_modules/lodash /tmp/lodash-experiment
# edit freely in /tmp/lodash-experiment
```

For permanent patches, use a dedicated patching workflow (planned for
v0.4) instead of in-place edits.

---

## 8. Backup considerations

What to back up and what to skip:

- `~/.guroku/cas` — **do not bother.** Every byte in here can be
  refetched from the npm registry. Backing it up just wastes backup
  storage. If you lose it, the next `guroku install` will rebuild
  what you need.
- `~/.guroku/cache/metadata` — **do not bother.** Pure cache.
  Regenerable from the network.
- Your project source — **back up.** This is your actual work.
- `package.json` — **back up.** It's source.
- `guroku.lock` — **back up, and commit it to version control.** It
  pins exact versions and integrity hashes for every transitive
  dependency. Without it, a future install can't reproduce the same
  tree.
- `node_modules` — **do not back up.** Regenerable from
  `package.json` + `guroku.lock` + the CAS.

---

## 9. Cross-machine portability

Do not copy `~/.guroku` or `node_modules` directories between
machines. Two reasons:

1. **Hardlinks don't transfer.** A hardlink is a second name for an
   inode, and inodes are filesystem-local. When `tar`, `rsync`, `cp`,
   or a cloud sync tool moves a hardlinked tree to a different
   filesystem, you usually end up with either independent copies of
   each file (massively inflating disk use) or broken references,
   depending on the tool's flags. Either way, the CAS-sharing
   property is lost.
2. **Path-sensitive symlinks.** Some of guroku's symlinks are
   relative and survive a copy, but others embed assumptions about
   the surrounding layout. Reproducing the layout from scratch on the
   destination is more reliable than copying it.

The right way to "move a project to another machine" is:

```sh
# On the destination machine, with package.json and guroku.lock
# already in place:
guroku install --frozen-lockfile
```

`--frozen-lockfile` refuses to update `guroku.lock` and reproduces
the exact dep tree the lockfile pins. This is also the recommended
mode in CI.

---

## 10. FAQ

**Can I share `~/.guroku/cas` between users on the same machine?**
Not supported in v0.3. The CAS is owned by the user that wrote it,
and guroku does not yet handle the permission model needed to share
it across UNIX users safely. Each user gets their own
`~/.guroku/cas`.

**Can I move `~/.guroku` somewhere else (e.g. to a different disk)?**
Not supported in v0.3. There is no `GUROKU_HOME` environment variable
or config option to relocate the store yet. As a workaround on Unix,
some users symlink `~/.guroku` to a directory on a larger disk before
their first install — this works but is not officially supported.
Native relocation via env var is tracked for v0.4.

**What happens if I run out of disk during an install?**
guroku writes new CAS entries via a tmp-then-rename pattern: the
extracted contents of a tarball go into a temporary directory first,
and only after every file is on disk does guroku atomically rename
the temp directory into its final hash-named location. If the install
runs out of disk midway, you'll see a clean error, the partially
written temp directory is cleaned up, and the CAS is left in a
consistent state. Free up some disk and re-run `guroku install`; it
will pick up where it left off.

**Why is `~/.guroku/store/` empty / missing on my machine?**
That directory is the v0.1/v0.2 layout and is unused in v0.3. If you
upgraded from an older guroku, you can delete it. If you installed
v0.3 fresh, it may not exist at all.

**Can I inspect a specific package version in the CAS?**
Yes. Look up the integrity hash in `guroku.lock` for the package
version you care about, then navigate to
`~/.guroku/cas/<first-2-hex>/<rest-of-hex>/`. Treat what you find
there as read-only (see section 7).
