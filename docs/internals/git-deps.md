# Git dependencies

guroku supports installing packages directly from a git repository, in
addition to the usual registry tarballs. This document describes how that
support is wired together internally.

## What's a git dep

A git dep is any `package.json` dependency whose value is a git URL or a
short-form git reference, rather than a semver range pointing at a registry
version.

Examples:

```json
{
  "dependencies": {
    "my-fork-a": "git+https://github.com/u/r.git",
    "my-fork-b": "git+https://github.com/u/r.git#v1.2.3",
    "my-fork-c": "github:u/r",
    "my-fork-d": "github:u/r#main"
  }
}
```

The optional `#<rev>` fragment pins a branch, tag, or commit SHA. When
absent, the default branch of the remote is used (whatever `HEAD` points at
on the server side).

## Classification

Spec parsing lives in `src/specs.rs`. A dependency string is matched
against the registry-version grammar first; anything that does not parse
as a semver range or `dist-tag` falls through into the git matcher, which
recognises:

- `git+https://`, `git+ssh://`, `git://`, `git+file://`
- `github:owner/repo`, `gitlab:owner/repo`, `bitbucket:owner/repo`
- a bare `<owner>/<repo>` shorthand

See `docs/internals/specs.md` for the full grammar and the precedence
order of the matchers. The output of classification is a `Spec::Git`
variant carrying a `GitRef { url, rev }` value, which is what the rest of
the install pipeline operates on.

## Cloning

Cloning is implemented in `src/git.rs`. The entry point is
`ensure_cloned(&GitRef) -> Result<PathBuf>`, which returns the path to a
working tree on disk.

guroku does not link against `libgit2`. It shells out to the system `git`
binary as a subprocess. There are two reasons for this:

1. The system `git` already knows how to talk to every protocol we care
   about, including `https` with a credential helper and `ssh` with the
   user's `~/.ssh` agent.
2. Bundling a git client is a large dependency (and a security surface)
   we do not need.

The clone strategy is a fast path with a fallback:

```rust
// Fast path: shallow clone of the named ref.
let mut cmd = Command::new("git");
cmd.args(["clone", "--depth", "1"]);
if let Some(rev) = &gref.rev {
    cmd.args(["--branch", rev]);
}
cmd.arg(&gref.url).arg(&target);

if cmd.status()?.success() {
    return Ok(target);
}

// Fallback: full clone, then checkout. This handles commit SHAs,
// which `--branch` cannot accept.
Command::new("git")
    .args(["clone", &gref.url])
    .arg(&target)
    .status()?;
Command::new("git")
    .current_dir(&target)
    .args(["checkout", rev])
    .status()?;
```

The fast path covers branch and tag refs. The fallback covers commit
SHAs, since `git clone --branch` does not accept a raw SHA. We prefer the
fast path because shallow clones are cheap and avoid pulling unrelated
history.

## Cache layout

Clones are reused across installs by hashing the URL into a stable
directory under the user cache:

```
~/.guroku/cache/git/<sha>/<safe-rev>/
```

- `<sha>` is the first 8 hex chars of `SHA-256(url)`. Eight chars is
  enough to make collisions astronomically unlikely while keeping paths
  short.
- `<safe-rev>` is the revision string (or `HEAD` if no rev was given)
  with every non-alphanumeric character replaced by `_`. This keeps the
  path filesystem-safe on Windows and avoids needing to URL-encode it.

For example, `git+https://github.com/u/r.git#v1.2.3` would land at
roughly:

```
~/.guroku/cache/git/3f2a1b9c/v1_2_3/
```

The same URL with `#main` would live in a sibling directory, so multiple
revs of the same repo coexist without re-cloning.

## Idempotency

After a successful clone, guroku writes an empty `.git-ready` marker
file at the working-tree root:

```
~/.guroku/cache/git/<sha>/<safe-rev>/.git-ready
```

`ensure_cloned` checks for this marker before doing any work; if it is
present, the function short-circuits and returns the existing path. The
marker is written last, so a half-finished clone (interrupted by Ctrl-C
or a network error) will not be mistaken for a complete one. On the next
run, the partial directory is removed and the clone is retried.

## Read into the resolver

Once the working tree exists, the resolver needs a `VersionInfo` for it
so the rest of the pipeline can treat it like any other package.

```rust
let path = git::ensure_cloned(&gref)?;
let manifest = Manifest::read_from(&path)?;

let version_info = VersionInfo {
    name: manifest.name.clone(),
    version: manifest.version.clone(),
    dependencies: manifest.dependencies.clone(),
    dev_dependencies: Default::default(),
    dist: Dist {
        tarball: "file:///guroku-local-source".into(),
        integrity: None,
        shasum: None,
    },
    ..Default::default()
};
```

The `dist.tarball` is a placeholder. The install pipeline never fetches
it because the corresponding `Resolved` carries `local_source =
Some(path)`, which short-circuits the fetcher. The placeholder exists so
the lockfile schema does not need a special-case nullable field.

## Linking

`into_linked_packages` is the bridge from `Resolved` (the resolver's
output) to `LinkedPackage` (the linker's input). For a git dep:

```rust
if let Some(src) = &resolved.local_source {
    LinkedPackage {
        id: resolved.id.clone(),
        source_dir: src.clone(),
        // ...
    }
}
```

The strict-layout linker (see `docs/internals/strict-layout.md`) then
hardlinks every file from `source_dir` into
`node_modules/.guroku/<id>/node_modules/<name>/`. End-user code in
`node_modules/<name>` is a symlink to the hardlinked tree, NOT directly
to the git clone. This matters for two reasons:

1. The clone in `~/.guroku/cache/git/...` is shared between projects.
   Letting one project's `node_modules` symlink straight into the cache
   would mean a `npm test` that writes into the package directory
   corrupts every other project using it.
2. Hardlinking decouples the lifetime of the per-project tree from the
   cache. `guroku cache clean git` can be run safely; existing
   `node_modules` keeps working until it is itself removed.

## Lockfile

Git deps are written into `guroku.lock` with their synthetic version:

```yaml
packages:
  "my-fork@1.4.0":
    resolved: "file:///guroku-local-source"
    git:
      url: "https://github.com/u/r.git"
      rev: "v1.2.3"
    dependencies:
      lodash: "4.17.21"
```

The `resolved` URL is the placeholder; the real reproducibility data is
in the `git` block. Reproducibility depends on the git ref being
immutable:

- A commit SHA is fully immutable. Strongly preferred.
- A tag is immutable in practice but can be force-pushed. Acceptable.
- A branch is mutable. The lockfile pins it at install time, but a
  re-clone after the upstream branch moves will produce different files.

`guroku install --frozen-lockfile` does not currently re-verify the
working-tree contents against a stored hash; this is on the v0.6 list.

## What we don't yet support

The following are known gaps in v0.5:

- `git+ssh://` URLs that prompt for a passphrase. The subprocess
  inherits a tty, so an interactive prompt will block the install
  silently from the user's perspective. The workaround is to use
  `ssh-agent` or a passphrase-less key.
- Submodules. The default fast-path clone is `--depth 1` and does not
  run `git submodule update --init`. Packages that genuinely need
  submodules at install time will be missing them.
- Building source-only packages. There is no equivalent of `npm pack`
  or the `prepare` lifecycle script yet, so a package whose published
  form is built from a `src/` directory will not produce its built
  output. For now, point at a tag whose tree already contains the
  built artefacts.

These are tracked for v0.5.x or v0.6.

## Auth

Because the system `git` is invoked as a subprocess, it inherits the
user's existing git authentication setup:

- `~/.git-credentials` (and any `credential.helper` configured in
  `~/.gitconfig`)
- `~/.ssh/` keys, including any keys held by `ssh-agent`
- Any `GIT_*` environment variables already in the parent shell

guroku does NOT pipe `_authToken` values from `.npmrc` into git. The
npm token namespace and the git credential namespace are separate, and
mixing them would silently leak registry tokens to git remotes. If a
private git repo needs auth, configure it through git's own mechanisms.

See `docs/internals/auth.md` for how registry auth works; the two
systems are deliberately decoupled.

## Diagnostics

Two knobs are available:

```sh
# Echo every git command line and its exit status.
GUROKU_LOG=debug guroku install

# Force a re-clone of one repo by deleting its cache subdir.
rm -rf ~/.guroku/cache/git/3f2a1b9c/v1_2_3/
guroku install
```

The cache layout is intentionally browsable. If a clone is in a wedged
state (for example, the network died mid-fetch and somehow the
`.git-ready` marker was written), removing the subdirectory is the
supported recovery path. There is no `guroku cache repair` command
because the cache is, by design, regenerable from the lockfile.
