# Lifecycle Scripts

This page is the user-facing reference for lifecycle scripts in guroku.
For the internal mechanics (process spawning, env var assembly, sandboxing
intent, etc.), see `docs/internals/lifecycle.md`.

## 1. What lifecycle scripts are

Lifecycle scripts are special script names you put in `package.json#scripts`.
guroku recognizes them and runs them at specific points during `guroku install`,
without you having to invoke them explicitly.

They are otherwise ordinary npm-style scripts: a string that is handed to a
shell.

```json
{
  "name": "my-app",
  "version": "1.0.0",
  "scripts": {
    "preinstall": "echo about to install",
    "postinstall": "node ./build.js",
    "prepare": "tsc -p ."
  }
}
```

guroku v0.4 understands four lifecycle names:

- `preinstall`
- `install`
- `postinstall`
- `prepare`

Anything else under `scripts` is just a regular script you can call with
`guroku run <name>`. It is never triggered automatically.

## 2. When each runs

The order during a single `guroku install` invocation is:

1. **`preinstall`** (root) — runs before any dependency is touched. The
   `node_modules` tree may not exist yet at this point.
2. **Resolver + downloads + linking** — guroku resolves the dependency graph,
   fetches tarballs from the cache or registry, and lays out `node_modules`.
3. **For each installed dep, in dependency order:** the dep's own
   `preinstall` -> `install` -> `postinstall` is run, best-effort. See section 3.
4. **`install`** (root) — runs after all dependencies are present and linked.
5. **`postinstall`** (root) — runs after `install`.
6. **`prepare`** (root) — runs last. This matches npm's convention: `prepare`
   is what you use to build a package after it has been installed from a git
   source, and it must run after `postinstall`.

If a script name is not present in `package.json#scripts`, that step is simply
skipped. There is no error and no warning.

## 3. Per-dep failure policy

Per-dependency lifecycle scripts run on a **best-effort** basis.

If a transitive dependency's `postinstall` exits non-zero, guroku:

- prints a warning to stderr identifying the package, the script name, and the
  exit code,
- continues with the rest of the install,
- exits 0 if everything else succeeds.

The reasoning: a typical `node_modules` has hundreds of postinstall hooks,
many of which try to download platform-specific binaries, build native addons,
or write to surprising locations. A single broken postinstall in some
transitive dep should not block your whole install. You usually do not even
use that package directly.

If you need to fail hard on per-dep script errors (for example in a
reproducible CI environment), the recommended pattern is:

```sh
guroku install --ignore-scripts
# then run only the lifecycle hooks you actually want, by hand:
( cd node_modules/some-pkg && npm run postinstall )
```

That way you decide which hooks are mandatory.

## 4. Root failure policy

Root lifecycle scripts (the four scripts in *your* `package.json`) are
**fatal**. If `preinstall`, `install`, `postinstall`, or `prepare` exits
non-zero:

- guroku stops immediately,
- prints the failing script name and exit code,
- exits with a non-zero status itself.

Your install is considered failed. `node_modules` may be in a partially
populated state; re-running `guroku install` after fixing the script is
expected to be safe.

## 5. Skipping scripts

You can disable lifecycle scripts entirely with:

```sh
guroku install --ignore-scripts
```

This skips **both** root lifecycle scripts and per-dep lifecycle scripts.
It does not affect manually invoked scripts (`guroku run <name>` still works).

Common reasons to use it:

- **CI builds you don't fully trust.** Lockfile-based installs in CI are a
  common attack target; turning off scripts removes the most common foothold.
- **Faster local dev cycles.** When you are iterating on something unrelated
  to the build step, skipping `postinstall`/`prepare` can shave seconds off
  every install.
- **Recovering from a broken postinstall** in a dep, while you investigate.

## 6. Forwarding args

Lifecycle scripts take **no user input**. There is no flag to pass extra
arguments to `preinstall`/`install`/`postinstall`/`prepare`.

If you want a script that accepts args, define it as a regular script and
invoke it directly:

```sh
guroku run build -- --release --target wasm32
```

Anything after `--` is forwarded to the script verbatim.

## 7. PATH inside lifecycle scripts

guroku prepends the relevant `.bin` directories to `PATH` so installed CLI
tools are callable directly, the same way npm does.

- **Root scripts** (`preinstall`/`install`/`postinstall`/`prepare` of your
  own `package.json`):
    - `<project>/node_modules/.bin/` is prepended to `PATH`.
- **Per-dep scripts** (a dependency's own `preinstall`/`install`/`postinstall`):
    - `<project>/node_modules/.bin/` is prepended.
    - `<package>/node_modules/.bin/` (the dep's own nested bin dir, if any) is
      also prepended.

Practical consequence: a `postinstall` like

```json
{ "scripts": { "postinstall": "tsc -p ." } }
```

works as long as `typescript` is reachable in the dep tree. You do not need
`./node_modules/.bin/tsc` or `npx tsc`.

The rest of the environment is inherited from the parent process unchanged
(see section 10 for what guroku does *not* inject yet).

## 8. Cross-platform

guroku runs each lifecycle script through a shell:

- On Unix-like systems (macOS, Linux): `sh -c "<script>"`.
- On Windows: `cmd /c "<script>"`.

This means the script string is interpreted by the platform's default shell,
**not** by bash. Avoid bash-only constructs:

- `[[ ... ]]` (use `[ ... ]`)
- `<(...)` process substitution
- arrays, `${var,,}` case-folding, etc.

If you need real shell logic, put it in a portable helper file and call it:

```json
{
  "scripts": {
    "postinstall": "node ./scripts/postinstall.js"
  }
}
```

A Node script is portable across all guroku-supported platforms; a `bash`
one-liner in `package.json` may not be.

## 9. Security

Lifecycle scripts execute **arbitrary code from the registry**, with the same
privileges as the user running `guroku install`. Historically this is the
single largest attack vector in the npm ecosystem: malicious `postinstall`
hooks have been used to exfiltrate credentials, plant persistence, and
ransom developer machines.

Recommendations:

- **Use `--ignore-scripts` in CI** when you can. CI typically does not need
  `postinstall` to download a native binary if you cache `node_modules` or
  prebuild artifacts.
- **Read the postinstall hooks of new deps** before adding them, especially
  for low-popularity packages or packages that appeared recently. `guroku why
  <pkg>` plus a glance at `node_modules/<pkg>/package.json` is usually enough.
- **Audit `guroku.lock` changes carefully.** New transitive deps mean new
  lifecycle scripts you have not seen before. A lockfile diff in code review
  is the cheapest place to catch a supply-chain attack.

For more on the threat model and what guroku does and does not protect
against, see `docs/internals/security-model.md`.

## 10. What env vars guroku does NOT yet set

npm sets a large number of environment variables before invoking each
lifecycle script:

- `npm_lifecycle_event` (the script name being run)
- `npm_package_name`, `npm_package_version`
- `npm_package_*` for every field in the package's `package.json`
- `npm_config_*` for every npm config key
- and many more

**guroku v0.4 does not export any of these.** Your script sees only the
environment of the parent process, plus the adjusted `PATH` from section 7.

Some legacy postinstalls rely on these variables and will misbehave or fail
under guroku. Known examples include older versions of `node-gyp` consumers,
some telemetry-style postinstalls, and a handful of monorepo helpers that
detect "am I being installed?" by reading `npm_lifecycle_event`.

This gap is tracked for the v0.4.x series. The plan is to ship the
`npm_lifecycle_event`, `npm_package_name`, and `npm_package_version` first,
since those cover the vast majority of compatibility complaints, and to ship
the long tail of `npm_package_*` fields in a follow-up.

## 11. FAQ

**Can I run a lifecycle script manually?**

Yes. Lifecycle scripts are still ordinary scripts as far as `guroku run` is
concerned:

```sh
guroku run preinstall
guroku run postinstall
guroku run prepare
```

When invoked this way, none of the install machinery runs around them; only
the script body itself executes.

**Does `guroku add` run lifecycle hooks?**

Yes. `guroku add <pkg>` resolves and installs the new dependency, updates
`package.json` and `guroku.lock`, and then runs the same lifecycle sequence
as `guroku install`: per-dep scripts for any newly added deps, followed by
root `install` / `postinstall` / `prepare`. Use `--ignore-scripts` to skip
them, just like with `install`.

**What about `prepublish` / `prepublishOnly`?**

Those are npm-specific publish-time hooks. guroku v0.4 does not implement
`guroku publish` at all, so neither hook is recognized. If you publish via
`npm publish` from a guroku project, npm itself will still honor them; guroku
simply ignores them during install.

**What about `prepack` / `postpack`?**

Same answer: pack/publish are out of scope for v0.4. The hooks are silently
ignored.

**Why does my `postinstall` see a different `cwd` than under npm?**

guroku runs each script with its `cwd` set to the directory of the
`package.json` that declared it. For the root script that is your project
root; for a per-dep script that is `node_modules/<pkg>`. This matches npm.
If you are seeing a different working directory it is almost certainly
because the script itself `cd`s somewhere.
