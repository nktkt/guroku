# Git Dependencies

guroku v0.5 can install packages directly from a git repository instead of
the npm registry. This is useful for forks, unpublished packages, internal
mirrors, and pinning to a specific commit of an upstream project.

This page covers the supported spec forms, what guroku does behind the
scenes, where the clone lives on disk, and how to deal with the common
sharp edges (auth, refreshing, build steps).

## Spec forms

A git dependency is declared in the `dependencies` (or `devDependencies`,
`optionalDependencies`) field of `package.json` like any other package.
The version string identifies it as a git spec.

```json
{
  "dependencies": {
    "my-fork":   "git+https://github.com/u/r.git",
    "pinned":    "git+https://github.com/u/r.git#v1.2.3",
    "by-commit": "git+https://github.com/u/r.git#abc1234",
    "short":     "github:u/r",
    "short-br":  "github:u/r#main",
    "private":   "git+ssh://git@gitlab.example.com/u/r.git"
  }
}
```

The supported forms in detail:

- `"my-fork": "git+https://github.com/u/r.git"` — clone HEAD of the
  repository's default branch. Whatever the remote considers `HEAD` at
  install time is what you get.
- `"my-fork": "git+https://github.com/u/r.git#v1.2.3"` — clone the tag
  or branch named `v1.2.3`. guroku does not distinguish between tag and
  branch refs; it passes the value through to git.
- `"my-fork": "git+https://github.com/u/r.git#abc1234"` — clone the
  repository, then `git checkout abc1234`. Anything that isn't a tag or
  branch is treated as a commit-ish.
- `"my-fork": "github:u/r"` — shorthand that expands to
  `https://github.com/u/r`. Equivalent to the `git+https://...` form
  above.
- `"my-fork": "github:u/r#main"` — shorthand with a ref. Expands to
  `https://github.com/u/r` checked out at `main`.
- `"my-fork": "git+ssh://git@gitlab.example.com/u/r.git"` — SSH URLs are
  supported. Authentication uses your normal `~/.ssh` setup (see Auth
  below).

Other host shorthands (`gitlab:`, `bitbucket:`) are not yet recognised
in v0.5. Use the explicit `git+https://` or `git+ssh://` form for
non-GitHub hosts.

## What guroku does

guroku does not embed a git client. It invokes the system `git` binary
as a subprocess and shells the actual fetch out to it.

The strategy is:

1. Try a shallow clone of just the requested ref:

   ```sh
   git clone --depth 1 --branch=<ref> <url> <target>
   ```

   When no ref is specified, the `--branch` flag is omitted and git
   clones the default branch.

2. If the shallow clone fails — for example, because `<ref>` is a commit
   SHA rather than a branch or tag, and the remote does not allow
   shallow fetching by SHA — guroku falls back to a full clone followed
   by `git checkout`:

   ```sh
   git clone <url> <target>
   git -C <target> checkout <ref>
   ```

This two-step strategy means commit SHAs work without requiring you
to think about whether the remote supports `uploadpack.allowReachableSHA1InWant`.
It is slower than the shallow path, but it always converges.

`git` must be on `PATH`. guroku does not ship its own copy.

## Where the clone lives

Cloned repositories are kept under the guroku cache:

```
~/.guroku/cache/git/<sha>/<safe-rev>/
```

- `<sha>` is the SHA-256 of the resolved git URL (after shorthand
  expansion).
- `<safe-rev>` is the requested ref with unsafe characters replaced, or
  a sentinel like `_HEAD` when no ref was specified.

The directory structure is content-addressed by `(url, ref)`, which
means two `package.json` entries that point at the same URL and ref —
even across different projects on your machine — share the same clone.
This keeps installs cheap when you have several projects depending on
the same fork.

## Editing or refreshing

guroku does not run `git pull` on subsequent installs. Once a clone
exists in the cache for a given `(url, ref)`, that clone is reused
forever, even if the remote branch has moved on.

This is deliberate: it keeps installs deterministic on machines where
the lockfile pins a specific commit. It does mean that if you depend on
a moving ref like `#main`, you have to refresh manually.

To re-fetch a single dependency, find its cache directory and remove
it:

```sh
rm -rf ~/.guroku/cache/git/<the-sha>
guroku install
```

The `<the-sha>` is the directory name under `~/.guroku/cache/git/`. You
can look it up by running `ls -la ~/.guroku/cache/git/` and matching by
modification time, or by re-running install with `--verbose` which
prints the cache key.

To refresh every git dependency in one go:

```sh
rm -rf ~/.guroku/cache/git
guroku install
```

This nukes the entire git cache. The next `install` repopulates it from
scratch.

## Pinning to commit SHAs

For anything you actually depend on in production, pin to a commit SHA
rather than a branch or tag:

```json
{
  "dependencies": {
    "my-fork": "git+https://github.com/u/r.git#9f3c1ab"
  }
}
```

A branch like `main` can move under you between installs on different
machines (since guroku caches per-machine). Tags are conventionally
immutable but technically can be force-pushed. Commit SHAs cannot
change meaning.

When you bump the dependency, change the SHA in `package.json` and
re-install. The lockfile records the resolved SHA either way, but
having it in `package.json` itself is clearer for code review.

## Auth

guroku does not pipe credentials into git. Whatever auth your normal
`git clone` of the same URL would use, that's what the install gets.

For HTTPS URLs:

- `~/.git-credentials`, populated by `git config credential.helper store`.
- Any configured `credential.helper` (osxkeychain on macOS, manager on
  Windows, libsecret on Linux, etc.).
- GitHub CLI's `gh` credential helper if you've run `gh auth login`.

For SSH URLs:

- `~/.ssh/config` for host-specific settings (port, identity file,
  proxy).
- `ssh-agent` for unlocked keys.
- Per-host known_hosts.

Test your config independently by running the same `git clone` command
that guroku would run. If `git clone <url>` works in a fresh shell,
guroku's install will work.

## Building source-only packages

A git repository is a source tree, not a published tarball. Whether
guroku can install it usefully depends on what's in the tree.

Repos that ship a built `dist/` (or whatever the package's `main`/
`exports` point at) work out of the box. guroku copies the working tree
into `node_modules/<name>` and the `require`/`import` paths resolve
normally.

Repos that build their published artifact from source — TypeScript
compiled to JavaScript, a Rollup bundle, etc. — need to declare a
`prepare` script:

```json
{
  "scripts": {
    "prepare": "tsc -p ."
  }
}
```

guroku v0.5 runs `prepare` for git dependencies after the clone is in
place. This matches npm's convention: the `prepare` script is the
designated hook for "make this checkout into a usable package".

If the `prepare` script itself requires devDependencies to be installed
first (the typical case for TypeScript), the multi-stage process is
fragile: guroku has to install the dep's own deps including dev ones,
run `prepare`, then prune dev deps. This works in straightforward cases
but quickly breaks down with workspaces, peer dependency cycles, or
devDeps that themselves want to build.

For anything beyond a trivial build step, the cleanest answer is
publishing the package — to npm, to a private registry, or to a tarball
URL — rather than depending on the git source.

## What v0.5 doesn't yet support

The following are recognised gaps. They are tracked but not implemented
for v0.5:

- **Submodules.** Clones use `--depth 1` by default and submodules are
  not initialised. If your dependency uses submodules to vendor part of
  itself, the install will produce a tree with empty submodule
  directories.
- **Interactive credential prompts.** If git decides to prompt for a
  username or password, the subprocess blocks waiting for stdin and the
  install hangs. Configure a credential helper or SSH key beforehand.
- **Sparse checkouts.** There is no way to clone only one subdirectory
  of a monorepo. The whole repo is fetched.
- **HTTPS with embedded credentials in the URL.** Spec strings like
  `git+https://user:token@host/u/r.git` work because git itself
  accepts them, but please don't: the credentials end up in
  `package.json`, the lockfile, and any logs.

## Common errors

### `git command failed: ... fatal: ...`

The subprocess returned non-zero. The line after the colon is git's own
error message, passed through unchanged. The two most common causes:

- **Auth.** `fatal: Authentication failed` or `Permission denied
  (publickey)` — your credential helper or SSH agent isn't set up for
  this host. Reproduce with a plain `git clone` and fix it there.
- **Non-existent ref.** `fatal: Remote branch <ref> not found in
  upstream origin` or `error: pathspec '<sha>' did not match` — the
  branch, tag, or commit you asked for doesn't exist on the remote.
  Check spelling and that the ref has actually been pushed.

### `file dependency <name> has no readable package.json`

The clone succeeded, but the repository root has no `package.json`.
This is most often a monorepo where the package you actually want
lives under `packages/<name>/` or similar.

guroku v0.5 does not yet support a path-into-clone field, so there is
no way to say "clone this repo, but treat `packages/foo` as the
package root". Workarounds:

- Fork the monorepo and add a top-level `package.json` that re-exports
  the subpackage.
- Use a separate publish step (`npm publish` from the subpackage
  directory) and depend on the published version.

## CI considerations

For CI, the same constraints apply as on a developer machine, but it's
easy to forget them in a fresh container:

- `git` must be on `PATH`. Most CI base images include it; a few
  minimal ones (some Alpine variants, distroless) do not.
- Any private repos you depend on need credentials. For SSH, mount an
  SSH key and start an `ssh-agent`. For HTTPS, configure a credential
  helper or use a token URL — and make sure the token is injected via
  CI secrets, not committed in `package.json`.
- The git cache lives under `~/.guroku/cache/git`. If your CI caches
  `~/.guroku/cache`, git deps benefit from the same caching as registry
  packages. If the cache is keyed on the lockfile, refreshing a moving
  branch ref still requires invalidating the cache manually.
- Shallow clones are fastest. Most cases use the shallow path; only
  pinning to a commit SHA on a server without `allowReachableSHA1InWant`
  forces the full clone.

If a git install works locally but fails on CI, run the same `git clone`
command directly inside the CI environment. The failure mode is almost
always reproducible at the git level.
