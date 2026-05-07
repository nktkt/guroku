# Storage

This document describes guroku's on-disk storage model: where downloaded packages
live, how they get into a project's `node_modules`, and how that is expected to
evolve over the next few releases.

## 1. Purpose

guroku is a Rust npm-style package manager. Like pnpm and bun, it draws a hard
line between two concerns:

- **The store** — what we actually downloaded and extracted from the registry.
  This is global to the user (or machine), shared across every project.
- **The linker output** — what your project's `node_modules` looks like. This is
  per-project and is derived from the store by a "linking" step.

Keeping the two separate is the central idea. The store is canonical and never
mutated by a project install. The linker reads from the store and produces a
project-local view of the dependency graph. That separation is what enables the
space-saving and isolation properties pnpm popularized.

In v0.1, both pieces exist but the linker is deliberately naive (it copies). The
shape of the store is what later versions will build on.

## 2. v0.1 layout

All guroku state lives under a single root in the user's home directory:

```
~/.guroku/                              # root
~/.guroku/store/<name>/<version>/       # extracted packages
~/.guroku/cache/tarballs/                # reserved for raw .tgz cache (unused in v0.1)
```

The `cache/tarballs/` directory is created but not yet populated. It is the
intended home for the raw `.tgz` files we fetch from the registry once we start
caching them separately from the extracted form (so that re-extraction or
re-hashing is possible without a network round trip).

### Scoped packages

npm scoped names like `@types/node` contain a forward slash. Putting that on
disk verbatim would mean every scoped package adds an extra directory level
(`store/@types/node/...`), which complicates iteration, lookup, and any future
content-addressable scheme that wants a flat key.

guroku replaces the `/` in a scoped name with `+`:

```
@types/node          ->  ~/.guroku/store/@types+node/<version>/
@babel/core          ->  ~/.guroku/store/@babel+core/<version>/
```

The `+` character is not legal in npm package names, so the encoding is
unambiguous and reversible. The result is one directory level per package,
scoped or not.

## 3. What lives in a store package directory

The contents of `~/.guroku/store/<name>/<version>/` are the contents of the
package's npm tarball with the leading `package/` segment stripped.

npm tarballs are conventionally laid out as:

```
package/
  package.json
  index.js
  lib/...
  README.md
```

After extraction into the store, `package/` is gone:

```
~/.guroku/store/lodash/4.17.21/
  package.json
  index.js
  lib/...
  README.md
```

So `package.json` lands directly at the root of the store directory. This is
what every consumer (linker, resolver, future tooling) expects, and it matches
how the package will appear inside `node_modules/<name>/`.

## 4. Linking in v0.1: `link_flat`

The v0.1 linker is a function called `link_flat`. For each package the
resolver decided to install, it does the simplest thing that works:

1. Take the store path: `~/.guroku/store/<name>/<version>/`.
2. Recursively **copy** every file into `<project>/node_modules/<name>/`.

That is the entire algorithm. There are no symlinks, no hardlinks, no
content-addressable indirection, no nested `node_modules`. After linking, the
project's `node_modules` looks like a flat npm-style layout, and every byte has
been duplicated from the store.

This is intentional for v0.1:

- It is trivial to reason about.
- It works on every filesystem (no symlink or hardlink support required).
- It produces a `node_modules` that any Node.js resolver will accept without
  surprises.

It is also obviously not space-efficient. Installing the same dependency in ten
projects copies it ten times. Fixing that is the job of v0.3.

## 5. Linking in v0.3 (target)

The roadmap target for v0.3 is to switch the store and the linker to a real
content-addressable design, similar to pnpm:

- **Store keying.** Store entries are keyed by the SHA-512 of the tarball, not
  by `<name>/<version>`. Two packages whose tarballs hash to the same value
  share the same on-disk bytes. The `<name>/<version>` form becomes a lookup
  index over the content-addressed store rather than the storage layout itself.
- **Per-package isolation directory.** Each `<name>@<version>` gets a dedicated
  directory inside the project:

  ```
  <project>/node_modules/.guroku/<name>@<version>/node_modules/<name>/
  ```

  The files inside that innermost `<name>/` directory are **hardlinks** back to
  the content-addressed store. Hardlinks share inodes, so disk usage stays
  proportional to unique content, not to (projects x dependencies).
- **Top-level symlinks.** At the top of `node_modules`, each direct dependency
  is a symlink pointing into its corresponding `.guroku/<name>@<version>/...`
  directory.

### The phantom-dependency problem

In a classic flat `node_modules`, every transitive dependency ends up as a
sibling of the packages you actually depend on. Node.js resolution will happily
walk up and find them, so your code can `require('some-transitive')` even
though it is not listed in your `package.json`. That works until the day a
resolver change, a hoist change, or a version bump removes the package from the
top level, and your code breaks for no obvious reason. This is the
phantom-dependency problem.

The v0.3 layout fixes it the same way pnpm does: the only things visible at the
top of `node_modules` are the packages you actually declared. Each declared
dependency is a symlink into its own isolation directory, and that isolation
directory only sees the dependencies that package itself declared. Code that
imports a package it did not declare simply fails to resolve, immediately and
loudly, on the first install.

## 6. Concurrency and atomicity

The v0.1 store is **not** safe against concurrent writers.

If two `guroku` processes try to install the same `<name>@<version>` at the
same time, they can both observe that the store directory does not exist and
both start extracting into it. The result is a partially merged or corrupted
store entry. There is no lock and no atomicity guarantee.

In practice this is rare (most users run one install at a time) but it is a
real footgun for CI matrices and monorepo tooling that fan out installs in
parallel. It is documented here as a known issue.

The v0.3 plan is to adopt **write-then-rename**:

1. Extract into a temporary directory, e.g. `~/.guroku/store/.tmp/<random>/`.
2. Once extraction completes successfully, atomically `rename(2)` it into its
   final `~/.guroku/store/<name>/<version>/` path.
3. If the destination already exists at rename time, discard the temporary
   directory; the other writer won.

`rename(2)` is atomic on every supported filesystem when source and destination
are on the same filesystem, which they are by construction here. That gives us
"at most one writer's bytes are ever observed" without needing an explicit
lock.

## 7. Eviction

v0.1 does not evict anything from the store. Every package version ever
installed by any project on this machine stays in `~/.guroku/store/` forever.
The store grows monotonically.

This is fine for early use but obviously not a long-term answer. v0.4 is the
tentative target for a `guroku store gc` subcommand that walks all known
projects (or a user-supplied list), computes the set of `<name>@<version>`
entries actually referenced, and removes the rest. Until that lands, the
intended workaround is `rm -rf ~/.guroku/store` followed by reinstalling.

## 8. Diagram

The mapping between a project's `node_modules` and the global store in v0.1:

```
~/.guroku/                                    <project>/
|                                             |
+-- store/                                    +-- node_modules/
|   |                                         |   |
|   +-- lodash/                               |   +-- lodash/
|   |   +-- 4.17.21/                  copy    |   |   +-- package.json
|   |       +-- package.json   ------------>  |   |   +-- index.js
|   |       +-- index.js                      |   |   +-- lib/...
|   |       +-- lib/...                       |   |
|   |                                         |   +-- @types+node ......... (store key)
|   +-- @types+node/                          |       (linked as)
|   |   +-- 20.10.0/                          |   +-- @types/
|   |       +-- package.json   -- copy -->    |       +-- node/
|   |       +-- index.d.ts                    |           +-- package.json
|   |                                         |           +-- index.d.ts
|   +-- react/                                |
|       +-- 18.2.0/                           +-- @types/
|           +-- package.json   -- copy -->        +-- node/
|           +-- index.js                              +-- package.json
|                                                     +-- index.d.ts
+-- cache/
    +-- tarballs/                  (reserved, empty in v0.1)
```

Two things to notice:

1. The store uses `@types+node` (the `+`-encoded form) as a single directory
   level, but the linker writes it back out as `@types/node` so Node.js
   resolution sees a normal scoped layout.
2. Every arrow labelled "copy" is a full recursive file copy in v0.1. In v0.3
   those arrows become hardlinks into a content-addressed store, with an extra
   `.guroku/<name>@<version>/` indirection layer for isolation.
