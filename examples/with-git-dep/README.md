# with-git-dep

A guroku v0.5 example demonstrating installation of a dependency sourced
from a `git+https://` URL.

## What this example shows

How guroku v0.5 resolves and installs a dependency declared as a git
spec in `package.json`. Specifically, a spec of the form:

```
git+https://<host>/<owner>/<repo>.git#<revision>
```

guroku clones the repo, reads its `package.json`, and installs it as a
local-source dependency.

## Heads-up: substitute a real repo

The URL in this example's `package.json`:

```
git+https://github.com/example/guroku-fixture-tiny.git#main
```

is **intentionally fictional**. It will not resolve. Before running the
example, replace the URL with one of:

- A real repo you control or trust.
- A published demo, e.g.:

  ```
  git+https://github.com/sindresorhus/cowsay.git#main
  ```

  (assuming cowsay's repo is public and has a `package.json` at its
  root).

## Try it

After substituting a real URL in `package.json`:

```sh
cd examples/with-git-dep
rm -rf node_modules guroku.lock
guroku install
```

## What guroku does

1. Reads the spec, classifies it as
   `Git { url, revision: Some("main") }`.
2. Calls
   `git clone --depth 1 --branch=main <url> ~/.guroku/cache/git/<sha>/main/`.
3. Reads the cloned repo's `package.json`.
4. Installs the package as a local-source dependency. There is no CAS
   storage and no integrity check for git deps.

## Auth for private git repos

guroku shells out to the system `git` binary. Authentication is
delegated entirely to git's own configuration:

- **SSH:** keys under `~/.ssh/` (use a `git+ssh://` URL).
- **HTTPS:** credentials in `~/.git-credentials` or via your configured
  credential helper.

guroku does **not** pipe `_authToken` from `.npmrc` into git. npm-style
auth tokens apply only to registry requests.

## Pinning to a commit SHA

Strongly recommended for reproducibility. Branches like `main` can move
under you, and guroku does not record git revisions in a way that
re-resolves to the same tree if the branch advances. Pin to a SHA:

```json
{
  "dependencies": {
    "guroku-fixture-tiny": "git+https://github.com/example/repo.git#abc1234def"
  }
}
```

## Refreshing

guroku does **not** run `git pull` on subsequent installs. Once a
revision is in the cache, it is reused. To force a re-clone:

```sh
rm -rf ~/.guroku/cache/git
guroku install
```

## Common errors

- `git command failed: ... Authentication failed`
  Your git credentials are not set up for the host. Configure SSH keys
  or an HTTPS credential helper.

- `file dependency at ... has no readable package.json`
  The cloned repo's root contains no `package.json`. Many monorepos
  place packages under a subdirectory; guroku v0.5 does not yet support
  a path-into-clone field, so only repos with a top-level
  `package.json` work.

- `terminal prompts disabled`
  guroku invokes git non-interactively. Configure auth that does not
  require a TTY prompt: an SSH key with no passphrase (or an active
  agent), or a credential helper that returns credentials silently.

## Related docs

- `docs/git-deps.md`
- `docs/internals/git-deps.md`
