# Symlinks in the v0.3 Linker

This document describes how guroku's linker chooses symlink targets when
materializing a `node_modules/` tree on disk. It covers the rationale for
using symlinks, the precise shape of the targets, the relative-path
computation, platform quirks, and corner cases such as cycles.

The linker code referenced here lives under `crates/guroku-linker/` in
the `populate_node_modules` family of functions. The CAS (content
addressable store) details are covered separately in `cas.md`; the
strict layout that the linker materializes is described in
`strict-layout.md`.

## 1. Why Symlinks At All

guroku stores every package exactly once on disk, in the per-store CAS,
and the project-local `.guroku/` directory is the staging area where
extracted package directories live. Each package directory under
`.guroku/<name>@<version>/` is the canonical location for that
particular `(name, version)` pair within the project.

The strict layout requires that:

- Direct dependencies appear at `node_modules/<name>`.
- A package's own dependencies appear at
  `.guroku/<parent>@<v>/node_modules/<dep>`.

Both of these are *references* into the canonical location. We need a
filesystem primitive that lets one path point at another *directory*.
The two candidates are:

- **Hardlinks.** Hardlinks cannot point at directories on the
  filesystems guroku targets (APFS, ext4, NTFS). They are a
  per-file-content alias, not a directory alias. The CAS does use
  hardlinks for individual files, but that does not help us redirect a
  directory entry.
- **Symlinks.** Symlinks can point at directories. They are resolved
  lazily by the kernel when a path is traversed. Node.js's module
  resolver follows symlinks transparently during `require`.

Symlinks are therefore the only viable mechanism to make
`node_modules/<name>` point at a directory inside `.guroku/`.

## 2. Relative, Not Absolute

Every symlink the linker creates uses a **relative** target. The linker
never embeds the project's absolute path into a symlink.

The reason is mobility. A `node_modules/` tree built with relative
symlinks can be:

- Moved to a different directory.
- Renamed alongside the project.
- Copied to a sibling worktree.

...and the symlink graph still resolves correctly, because every link
is expressed in terms of "go up N levels, then descend into
`.guroku/...`". None of those operations changes the structure *within*
the tree.

There is one important caveat. The package files themselves are not
inside `node_modules/`; they are hardlinks into the per-user CAS, which
lives outside the project. Copying a `node_modules/` directory *across
machines* (for example via tar over SSH) carries the symlinks but not
the CAS-backed file content. On the destination machine the symlinks
still point at the right relative paths, but the files those paths
ultimately resolve to are gone. The fix is a fresh `guroku install`,
which re-links from the local CAS.

In short: relative targets buy you intra-machine portability, not
cross-machine portability.

## 3. The Two Symlink Shapes

The linker emits exactly two shapes of symlink. Distinguishing them is
useful when reading link traces or debugging by hand.

### 3.1 Top-Level Direct Dependency

A direct dependency `foo` of the project appears at:

```
node_modules/foo
```

and points at:

```
.guroku/foo@1.2.3/node_modules/foo
```

The relative target, computed from the link's *parent* directory
(`node_modules/`), is:

```
.guroku/foo@1.2.3/node_modules/foo
```

That is, it starts with `.guroku/`. There are zero `..` components,
because both the link and its target sit inside `node_modules/`.

### 3.2 Sibling Dependency

A dependency `bar` that `foo@1.2.3` depends on appears at:

```
.guroku/foo@1.2.3/node_modules/bar
```

and points at:

```
.guroku/bar@4.5.6/node_modules/bar
```

The relative target, computed from
`.guroku/foo@1.2.3/node_modules/` (the link's parent), is:

```
../../bar@4.5.6/node_modules/bar
```

Two `..` components: one to escape `node_modules/`, one to escape
`foo@1.2.3/`. Then we descend into `bar@4.5.6/node_modules/bar`.

### 3.3 Scoped Packages

Scoped packages introduce a small naming wrinkle. The package name
`@scope/name` contains a `/`, which we cannot use as a directory
component in `.guroku/<id>` because `<id>` would then have an inner
slash and confuse path operations.

The encoding rules are:

- The `.guroku/` segment uses `+` as a separator:
  `.guroku/@scope+name@1.0.0/`.
- The `node_modules/` interior uses the original `@scope/name`:
  `.guroku/@scope+name@1.0.0/node_modules/@scope/name`.

So a top-level direct dep of `@scope/name@1.0.0` is:

```
node_modules/@scope/name -> .guroku/@scope+name@1.0.0/node_modules/@scope/name
```

And a sibling dep where parent `foo@1.2.3` depends on `@scope/name`:

```
.guroku/foo@1.2.3/node_modules/@scope/name
    -> ../../@scope+name@1.0.0/node_modules/@scope/name
```

Note that the link's parent directory is
`.guroku/foo@1.2.3/node_modules/@scope/`, so escaping it requires
*three* `..` components when the dep is itself scoped:

```
.guroku/foo@1.2.3/node_modules/@scope/name
    -> ../../../@scope+name@1.0.0/node_modules/@scope/name
```

The relative-path computation handles this without special-casing,
because it operates on path components, not on package names.

## 4. The Relative-Path Computation

The core helper is `relative_to(target, base)`. It returns the path you
would write at a symlink whose containing directory is `base` so that
it points at `target`.

The algorithm is straightforward:

1. Split both paths into components.
2. Find the longest common prefix.
3. For each remaining component of `base`, emit `..`.
4. Append the remaining components of `target`.

### 4.1 Worked Example

Suppose the target is the canonical location of `B@1.0.0`, and the link
is going to be a sibling-dep entry under `A@1.0.0`:

```
target = node_modules/.guroku/B@1.0.0/node_modules/B
base   = node_modules/.guroku/A@1.0.0/node_modules
```

Step by step:

```
common = node_modules/.guroku
remaining(base)   = A@1.0.0/node_modules        -> 2 components
remaining(target) = B@1.0.0/node_modules/B      -> 3 components

rel = ../../B@1.0.0/node_modules/B
```

This is exactly what gets written to the symlink at
`node_modules/.guroku/A@1.0.0/node_modules/B`.

### 4.2 Edge Cases

- **Identical paths.** Both decompose the same way; there is nothing
  to link. The linker treats this as a programming error and panics in
  debug builds. It would only happen if two distinct graph nodes
  resolved to the same canonical directory, which the resolver
  prevents.
- **Disjoint paths.** If `base` and `target` share no prefix, the
  result is a stack of `..` followed by the absolute-style suffix of
  `target`. This does not happen in practice because both paths are
  constructed under the same project root.
- **Trailing slashes.** Stripped before splitting; the algorithm
  operates on components, not on textual paths.

## 5. Pre-Existing Entries

Before creating a symlink, the linker calls
`ensure_clean_for_symlink(path)`. This function removes whatever exists
at `path`, regardless of its kind:

- A regular file: unlinked.
- A directory: removed recursively (this is rare but can happen if a
  user manually copied files in).
- A symlink, valid or dangling: unlinked. (Note: removing a symlink
  removes the link itself, not its target.)

This is what makes a re-install with a changed version reachable from
the right path. If yesterday's install had `react@18.2.0` and today's
install resolves to `react@18.3.0`, the previous link
`node_modules/react -> .guroku/react@18.2.0/...` is replaced. The
package directory of the old version may still exist under `.guroku/`
until GC runs, but the link itself is rewritten in place.

The function is intentionally tolerant: a missing path is not an error,
because the most common case is a fresh `node_modules/` where nothing
was there to begin with.

## 6. Windows Quirks

Symlinks on Windows are not as uniform as on POSIX systems. The linker
has two Windows-specific behaviors.

### 6.1 File vs Directory Symlinks

Windows distinguishes `symlink_file` from `symlink_dir`, and the
distinction is enforced by the OS rather than inferred from the target.
The linker branches on `target.is_file()`:

```
if target.is_file() {
    std::os::windows::fs::symlink_file(target, link)
} else {
    std::os::windows::fs::symlink_dir(target, link)
}
```

For package-directory links (the only kind discussed in this document)
the branch always selects `symlink_dir`. The file branch exists only
for completeness; the v0.3 strict layout does not produce
file-targeted symlinks.

### 6.2 Permissions

Creating symlinks on Windows historically required administrator
privileges. As of Windows 10 1703, Developer Mode lifts that
requirement for the current user. guroku does not manage this
configuration; if the user is not an admin and Developer Mode is off,
`populate_node_modules` returns an `Io` error from the underlying
`CreateSymbolicLinkW` call.

The error message in this case is the OS-supplied "A required privilege
is not held by the client", which we surface verbatim. The CLI's error
formatter recognizes this string and prints a hint pointing at the
Windows Developer Mode toggle.

## 7. Tools That Resolve Symlinks

Several tools come into play when verifying a strict-layout install
by hand or in CI.

- **Node.js.** During `require` resolution, Node follows symlinks. By
  default it then sets `module.filename` to the *real* path
  (`realpath`-resolved), which is what makes `__dirname` of a linked
  module point at its `.guroku/` location rather than its
  `node_modules/` link. This is why the strict layout is transparent to
  user code: the canonical directory is always reachable both through
  the link and through the resolved path.
- **`realpath(1)`** on POSIX prints the fully resolved path, walking
  every symlink. Used like `realpath node_modules/foo` to confirm the
  CAS-backed underlying path.
- **`readlink -f`** does the same on Linux.
- **`readlink`** without `-f` prints only the immediate target
  (relative, in our case). Useful for inspecting one hop.
- **`ls -l`** shows the link arrow `link -> target`, with `target`
  rendered exactly as it was written, i.e. relative.

A common debugging workflow is:

```
ls -l node_modules/foo
readlink node_modules/foo
realpath node_modules/foo
```

The first two confirm what guroku wrote; the third confirms that the
CAS-backed directory exists.

## 8. Cycles

A package depending on itself transitively is uncommon but legal in
the npm ecosystem. For instance, `a -> b -> a` can occur when two
packages co-evolve. The strict layout handles this without producing a
symlink cycle at the filesystem level.

The reason is structural. Each package's own dependencies live inside
its own `.guroku/<id>/node_modules/`, which is not a parent of the
target. A self-loop traversal goes:

```
node_modules/a
  -> .guroku/a@1/node_modules/a            (link target: package dir)
.guroku/a@1/node_modules/b
  -> ../../b@1/node_modules/b              (sibling link)
.guroku/b@1/node_modules/a
  -> ../../a@1/node_modules/a              (sibling link, back to a)
```

Each individual link points at a real directory in `.guroku/`. The
links never form a closed loop because the inner
`node_modules/<dep>` link lives *outside* the parent's package
directory; the parent's package files are at
`.guroku/<parent>@<v>/...` minus the `node_modules/` segment, while the
sibling links are *inside* `.guroku/<parent>@<v>/node_modules/`. A
filesystem walker following symlinks can revisit the same canonical
directory, but it never recurses through a chain of symlinks that
returns to a directory it is currently inside.

Practically, this means tools like `find -L`, `du -L`, and recursive
copies see at worst repeated visits, not infinite recursion. Visit
deduplication, when needed, is done by inode in the consumer.

## See Also

- `strict-layout.md` for the overall directory shape produced by the
  linker.
- `cas.md` for how individual files are hardlinked from the per-user
  store.
- `dependency-graph.md` for how the resolver decides which
  `(name, version)` pairs end up under `.guroku/`.
