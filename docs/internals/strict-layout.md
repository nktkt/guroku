# Strict `node_modules` Layout (v0.3)

This document describes how guroku v0.3 materialises a `node_modules/` tree on
disk. The layout is deliberately modelled on pnpm's strict layout: every
package gets its own private `node_modules/` directory, and only declared
dependencies are visible inside it. The mechanism is what prevents *phantom
dependencies* — the class of bug where a library happens to work because a
sibling in the dependency graph happened to install the package it forgot to
declare.

If you have used pnpm, almost everything here will be familiar; the only real
difference is the prefix directory name (`.guroku/` instead of `.pnpm/`).

---

## 1. The shape

The high-level rule is:

> Every package in the dependency graph is materialised at exactly one path:
> `node_modules/.guroku/<name>@<version>/node_modules/<name>/`.

The `.guroku/` directory at the top of `node_modules/` is the *store view* —
think of it as the per-project flat list of every package the resolver decided
on. The `<name>@<version>/` suffix means a project can host any number of
versions of the same package side by side without collision.

The trailing `node_modules/<name>/` is what makes Node's resolver work: when
Node is inside `<name>` and it does `require('foo')`, it walks up looking for
`node_modules/foo`. Because every package lives one directory below its own
`node_modules/`, its peer-deps line up naturally.

A complete worked example (taken verbatim from
`tests/fixtures/expected_strict_layout.txt`):

```
node_modules/
├── .guroku/
│   ├── is-number@6.0.0/
│   │   └── node_modules/
│   │       └── is-number/
│   │           ├── package.json
│   │           └── index.js
│   ├── is-odd@3.0.1/
│   │   └── node_modules/
│   │       ├── is-odd/
│   │       │   ├── package.json
│   │       │   └── index.js
│   │       └── is-number -> ../../is-number@6.0.0/node_modules/is-number
│   └── lodash@4.17.21/
│       └── node_modules/
│           └── lodash/
│               ├── package.json
│               └── lodash.js
├── is-odd -> .guroku/is-odd@3.0.1/node_modules/is-odd
└── lodash -> .guroku/lodash@4.17.21/node_modules/lodash
```

Note three things in that tree:

- `is-number@6.0.0/node_modules/is-number/` has no sibling entries — `is-number`
  declares no runtime deps.
- `is-odd@3.0.1/node_modules/` *does* have a sibling: `is-number`, which is a
  symlink across into `is-number@6.0.0`. That's the strict-mode "only declared
  deps are visible" rule in action.
- The top of `node_modules/` only contains `is-odd` and `lodash` — the project's
  *direct* dependencies. `is-number` is intentionally not at the top, because
  the project itself never declared it.

---

## 2. What's a hardlink and what's a symlink

guroku uses two different kinds of links and it is important to keep them
straight:

| Path                                                          | Kind     | Points to                                              |
|---------------------------------------------------------------|----------|--------------------------------------------------------|
| `.guroku/<name>@<v>/node_modules/<name>/<file>`               | hardlink | the file's blob in the global CAS                      |
| `.guroku/<name>@<v>/node_modules/<sibling>`                   | symlink  | `../../<sibling>@<v>/node_modules/<sibling>`           |
| `node_modules/<name>` (direct dep of the project)             | symlink  | `.guroku/<name>@<v>/node_modules/<name>`               |

Concretely:

- **Files** inside the actual package directory are hardlinks. There is one
  copy of each unique file blob across the whole CAS, and every project that
  uses that file shares the same inode. This is where the disk-space win
  comes from.
- **Sibling-dep entries** inside a package's own `node_modules/` are
  *symlinks* — they point sideways into another `.guroku/<dep>@<ver>/...`
  directory. They have to be symlinks (not hardlinks) because directories
  cannot be hardlinked on any mainstream filesystem.
- **Top-level `node_modules/<name>`** entries for direct deps are also
  symlinks, again because they target a directory.

---

## 3. Why this prevents phantom dependencies

Node's module resolver, when asked for `require('lodash')` from inside a file
at `<X>`, walks up the directory tree looking for a `node_modules/lodash`
entry. It does *not* care whether `lodash` is listed in any `package.json` —
it only cares whether the directory exists.

In a flat (npm-style) layout, the top of `node_modules/` ends up being a
soup of every transitive dep. A library `foo` whose `package.json` only
lists `bar` can still successfully `require('lodash')` if `lodash` happens to
have been installed as a top-level dep of the project, because Node's
walk-up will find it. This is a *phantom dependency* — `foo` is silently
relying on a package it did not declare, and the day someone removes
`lodash` from the project, `foo` breaks.

With the strict layout, `foo` lives at
`.guroku/foo@<v>/node_modules/foo/`. The only `node_modules/` directory Node
will find when it walks up from inside `foo` is
`.guroku/foo@<v>/node_modules/`. That directory contains exactly the deps
`foo` declared, plus `foo` itself. There is no path for the walk to follow
that would let `foo` accidentally resolve `lodash`. The phantom dependency
has nowhere to come from.

---

## 4. Scoped packages

Scoped packages (`@scope/name`) introduce a subtlety: the scope is a real
subdirectory on disk (`node_modules/@types/node/`), but `.guroku/` is a flat
directory and we do not want one entry per scope inside it.

The convention is to encode `/` as `+` in the `.guroku/` directory name only:

```
node_modules/
├── .guroku/
│   └── @types+node@20.0.0/
│       └── node_modules/
│           └── @types/
│               └── node/
│                   ├── package.json
│                   └── index.d.ts
└── @types/
    └── node -> ../.guroku/@types+node@20.0.0/node_modules/@types/node
```

Three rules cover this:

1. The `.guroku/` directory name uses `+`: `@types+node@20.0.0`.
2. Inside that, the `node_modules/<name>/` part keeps the slash, because the
   real package needs to live at the path Node expects:
   `@types+node@20.0.0/node_modules/@types/node/`.
3. The top-level surface symlink also keeps the slash:
   `node_modules/@types/node -> ../.guroku/@types+node@20.0.0/node_modules/@types/node`.
   Note the extra `..` in the relative target compared to the unscoped case,
   because `@types/` is itself one directory deep.

---

## 5. Symlink targets are relative

Every symlink that guroku creates uses a *relative* path target. The
sibling-dep symlink in the worked example is

```
.guroku/is-odd@3.0.1/node_modules/is-number -> ../../is-number@6.0.0/node_modules/is-number
```

not

```
.guroku/is-odd@3.0.1/node_modules/is-number -> /Users/alice/proj/node_modules/.guroku/is-number@6.0.0/node_modules/is-number
```

This matters in three concrete cases:

- **Moving the project.** `mv project elsewhere/` does not break anything,
  because every symlink's target is expressed relative to its own location.
- **Sharing a checkout across machines via a network mount or container
  bind-mount.** Absolute paths would be wrong on the other side; relative
  paths just work.
- **Cloning a `node_modules/` tree (e.g. for a build cache).** A simple `cp
  -a` of the whole tree produces a working tree at the destination, as long
  as the underlying CAS is reachable.

The top-level symlinks at `node_modules/<name>` use `.guroku/...` (one level
down); the sibling symlinks inside a package's own `node_modules/` use
`../../<dep>@<v>/node_modules/<dep>` (two levels up, then back down).

---

## 6. What about Windows

Windows distinguishes file-symlinks from directory-symlinks at the moment
of creation. On Unix this is not a thing — `symlink(2)` does not care what
the target is, and `readlink` works either way. On Windows the wrong choice
produces a link that exists but resolves incorrectly, which then causes
mysterious "module not found" errors.

The linker code branches on `cfg(windows)` and uses
`std::os::windows::fs::symlink_dir` for the package directories (which is
all of the symlinks guroku creates, since every link target is a directory).
The non-Windows branch uses `std::os::unix::fs::symlink`.

There is one further wrinkle that no amount of code can paper over: creating
symlinks on Windows requires either Developer Mode to be enabled or the
process to be running with admin privileges. If neither is true, the
`symlink_dir` call fails with a permission error. This is a known
limitation of strict mode on Windows; we surface the underlying OS error
rather than trying to fall back to junctions or copies, because either
fallback would silently change the semantics in ways that are likely to
mask bugs.

---

## 7. Hardlink fallback

`fs::hard_link` is not infallible. It fails on:

- cross-filesystem links (CAS on one mount, project on another),
- filesystems that do not support hardlinks at all (exfat, FAT32, some
  network filesystems),
- some sandboxed environments where the cross-link is technically possible
  but disallowed by policy.

When it fails, the linker falls back to a plain copy. The user gets correct
files; they just lose the disk-space dedup benefit for those particular
files.

The relevant code branch looks like this:

```rust
fn link_or_copy(src: &Path, dst: &Path) -> io::Result<()> {
    match fs::hard_link(src, dst) {
        Ok(()) => Ok(()),
        Err(e) if is_cross_device(&e) || is_unsupported(&e) => {
            // CAS lives on a different filesystem, or the target FS
            // does not support hardlinks (exfat, FAT32, some net FS).
            // Fall back to a copy: correctness preserved, dedup lost.
            fs::copy(src, dst).map(|_| ())
        }
        Err(e) => Err(e),
    }
}
```

The fallback is per-file, not per-package: if only some files fail to link,
only those files are copied. We do not warn on this in normal output
because on filesystems where the fallback fires, it would fire for every
single file and the warning would become noise. It is visible at `--verbose`.

---

## 8. What `populate_node_modules` does NOT do (yet)

Worth being explicit about, because a few things people might expect are
not implemented in v0.3:

- **No garbage collection of stale `.guroku/` entries.** If you run
  `guroku install`, then edit `package.json` to remove a dep, then run
  `guroku install` again, the old `.guroku/<old>@<v>/` directory will
  still be on disk. The top-level symlink will be gone (so the dep is no
  longer reachable from Node's resolver), but the materialised package
  directory is leaked until the next `guroku store prune` or until the
  user does `rm -rf node_modules`. Planned for v0.4.
- **No top-level `package.json` shim.** Some tools expect to find a
  `package.json` *inside* `node_modules/` for various reasons. We do not
  write one. If this turns out to bite us, it is a one-line fix.
- **No `.bin/` symlinks for executables.** Packages that declare a `bin`
  field in their `package.json` are not exposed via `node_modules/.bin/`.
  Lifecycle scripts (`preinstall`, `postinstall`, `prepare`, etc.) are
  also not run. Both of these land in v0.4 together, because lifecycle
  scripts depend on `.bin/` being populated to be useful.

---

## 9. Comparison with pnpm

This layout is structurally identical to pnpm's. The only differences are
cosmetic:

- pnpm uses `.pnpm/` as the prefix directory; we use `.guroku/`.
- pnpm has a few extra files inside the prefix dir (e.g. `node_modules/.modules.yaml`)
  that we do not write; they are pnpm-internal and not load-bearing for the
  Node resolver.

If you understand pnpm's layout, you understand guroku's. Tooling that is
written to inspect a pnpm tree (e.g. some IDE plugins, some auditing tools)
can usually be pointed at a guroku tree by changing one path constant.

---

## 10. Diagnostics

A couple of one-liners are useful when something looks wrong:

```sh
# Show every symlink under node_modules and where it points.
find node_modules -type l | head

# List the materialised packages in the .guroku store view.
ls -la node_modules/.guroku | head
```

The first is the fastest way to see whether the surface symlinks at the top
of `node_modules/` are present and pointing into `.guroku/`. The second is
the fastest way to see what was actually materialised, including any stale
entries left by the absence of GC (see section 8).

For deeper inspection:

```sh
# Verify that two project trees share inodes for a CAS-backed file.
ls -li node_modules/.guroku/lodash@4.17.21/node_modules/lodash/lodash.js

# Show the resolved real path of a symlinked sibling dep.
readlink node_modules/.guroku/is-odd@3.0.1/node_modules/is-number
```

If `find node_modules -type l` returns nothing, the linker did not run, or
it crashed before creating any symlinks. If it returns the surface
symlinks but the targets do not resolve (`find -L ... -type d` shows
warnings), then the `.guroku/` entries themselves are missing — most often
because the previous install was interrupted and left a half-built tree.
The remedy is `rm -rf node_modules && guroku install`.
