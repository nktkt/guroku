# Lifecycle Script Ordering

Status: internals doc, v0.4
Audience: contributors and curious users

This document describes the precise order in which `guroku install`
runs lifecycle scripts (`preinstall`, `install`, `postinstall`,
`prepare`) for both the root project and the installed dependencies.
The ordering matches npm's documented behaviour; the implementation
choices, failure semantics, and environment exposure differ in a few
places that are called out below.

For the security implications of running third-party scripts, see
[security-model.md](./security-model.md).

---

## 1. The pipeline

The full timeline of a single `guroku install` invocation:

```
[root preinstall]
[resolver]
[parallel CAS fetch+verify]
[populate node_modules + .bin/ shims]
[for each installed pkg: pre/install/postinstall (warn on failure)]
[root install]
[root postinstall]
[root prepare]
```

Each block is sequential with respect to the blocks above and below
it. Inside `[parallel CAS fetch+verify]` we fan out across the
worker pool; inside `[for each installed pkg: ...]` we currently do
not (see "Concurrency" below).

The pipeline holds whether the user passed an explicit set of
packages, ran a bare `guroku install`, or invoked `guroku ci`. The
only differences for `ci` are that the resolver phase is skipped
(the lockfile is authoritative) and that script failures are fatal
rather than warnings.

---

## 2. Root vs. per-package distinction

There are two distinct execution contexts.

**Root scripts** run from the user's project directory --- i.e. the
directory that contains the `package.json` guroku is currently
operating on. This is also the process `cwd` at the time guroku was
invoked. Root scripts are the four standard hooks:

- `preinstall`
- `install`
- `postinstall`
- `prepare`

**Per-package scripts** run from inside the installed copy of the
dependency, specifically:

```
<.guroku>/<id>/node_modules/<name>/
```

where `<id>` is the content-addressed package identity used by the
CAS layer (see [cas.md](./cas.md)) and `<name>` is the package's
declared name. Per-package scripts are:

- `preinstall`
- `install`
- `postinstall`

`prepare` is *not* run for already-published tarballs at install
time; it is reserved for the consumer-side build case described in
section 4.

---

## 3. Order within root

The root project's hooks bracket the rest of the install:

1. `preinstall`
2. resolve + download + link (the middle of the pipeline above)
3. per-package `preinstall` / `install` / `postinstall`, one
   dependency at a time
4. `install`
5. `postinstall`
6. `prepare`

This is the same sequence npm uses, and we do not deviate from it.
Scripts written against npm's documented contract should behave
identically under guroku, modulo the env-var gap described in
section 7.

The motivation for keeping per-package scripts *between* the root's
`preinstall` and the root's `install` is straightforward: the root's
own `install` script may legitimately depend on artifacts produced
by a dependency's `postinstall` (a generated binary, a built native
addon, etc.), so dependency hooks must be resolved first.

---

## 4. Why `prepare` runs last

`prepare` exists, in npm, primarily for *git-installed* dependencies
and for packages being prepared for publish. The relevant case here
is "this package was installed from a git URL and there is no
`node_modules` folder yet" --- npm runs `prepare` at install time so
that build-from-source steps (TypeScript compilation, bundling,
etc.) execute on the consumer's machine.

For consistency with that contract:

- `prepare` is the *last* root hook in the pipeline, after
  `postinstall`.
- For published-tarball deps it is not run at install time at all.
- For git-installed deps (planned in v0.4.x) it will run after the
  package's own `install`/`postinstall` chain, which mirrors npm.

Mirroring npm's placement here is the explicit goal: it keeps user
expectations stable for packages that have a `prepare` script and
expect it to behave exactly as npm describes.

---

## 5. Per-package script failure: warn-and-continue

If a dependency's `preinstall`, `install`, or `postinstall` script
exits non-zero, guroku **does not abort the install**. Instead it:

1. Captures the script's stdout/stderr into the install log.
2. Emits a `tracing::warn!` line of the form

   ```
   WARN guroku::lifecycle: postinstall failed for foo@1.2.3 (exit 1); continuing
   ```

3. Marks the package as "installed but script-failed" in the in-
   memory install report (this is surfaced in the final summary).
4. Proceeds to the next package.

The rationale is pragmatic: a usable `node_modules` is more valuable
than a clean abort. The vast majority of postinstall failures in
real-world JS dependencies are optional native-binary downloads or
analytics/telemetry pings; failing the entire install over those
strands the user.

If you want strict behaviour (e.g. in CI), the recommended pattern
is:

```
guroku install --ignore-scripts
# then run scripts explicitly via your own tooling
```

This is also what `guroku ci` should be configured to do for
production builds where any unexpected behaviour should be a hard
failure. Per-package script failures during `guroku install` are
warnings only; the root project's own scripts are fatal.

---

## 6. `--ignore-scripts` semantics

The `--ignore-scripts` flag, when present:

- Skips **all four** root hooks (`preinstall`, `install`,
  `postinstall`, `prepare`).
- Skips **all** per-package hooks across the dependency graph.
- Has no effect on the resolver, CAS fetch, or `node_modules`
  population. Those steps run normally.

The flag is **sticky for that invocation only**. It is not written
to `.npmrc`, `guroku.toml`, the lockfile, or any per-project state
file. The next `guroku install` without the flag will run scripts
again. If a user wants persistent behaviour, that is a config
concern (see [npmrc.md](./npmrc.md)) and should be set by the user
explicitly.

This non-persistence is deliberate: a flag that silently disables
script execution for future invocations is a foot-gun for security
auditors trying to reason about what actually ran.

---

## 7. Environment passed to scripts

### `cwd`

- Root scripts: the project directory (the directory containing the
  root `package.json`).
- Per-package scripts: `<.guroku>/<id>/node_modules/<name>/`.

### `PATH`

guroku prepends one or two entries to `PATH` before exec:

- Root scripts: `<project>/node_modules/.bin` is prepended to the
  inherited `PATH`.
- Per-package scripts: both `<pkg>/node_modules/.bin` *and* the
  project's `node_modules/.bin` are prepended, with the per-package
  one first. This matches the resolution order users get from
  running scripts manually inside a dependency directory.

The rest of the environment is inherited from the parent guroku
process. We do not currently strip variables for sandboxing; that
is tracked under the security model.

### `npm_lifecycle_event` and friends

npm exposes a battery of environment variables to lifecycle
scripts: `npm_lifecycle_event`, `npm_package_name`,
`npm_package_version`, `npm_config_*`, and so on.

**guroku does not yet export these.** This is a known gap. Some
legacy postinstall scripts --- particularly older native-addon
installers --- branch on `npm_lifecycle_event` and will mis-behave
under guroku. Exporting the npm-compatible env block is planned
for **v0.4.x**.

Until then, packages that hard-require these vars can be worked
around by setting them in the user's shell or by skipping their
scripts with `--ignore-scripts` and running the install manually.

---

## 8. Concurrency

**Root scripts**: sequential, by definition. Each root hook blocks
the install until it returns. There is no scenario in which two
root hooks run concurrently.

**Per-package scripts**: also sequential today. We iterate the
install plan in topological order and run each package's
`preinstall` / `install` / `postinstall` chain to completion before
moving on to the next package.

It is tempting to parallelise this --- the CAS fetch and link
phases are heavily parallel, after all --- but pnpm explicitly runs
postinstalls serially for the same reason we do: real-world
postinstalls assume a quiescent filesystem and frequently write
into shared locations (`.bin/`, the package's own node_modules,
sometimes the root's). Running them concurrently surfaces races
that are very hard to reproduce after the fact.

We may revisit this in a later version --- specifically, parallel
execution of postinstalls that are statically declared as pure
(no filesystem writes outside the package directory) might be safe
--- but the default will likely remain serial.

---

## 9. Logging

Every lifecycle script that guroku is about to execute is logged at
`info` level through `tracing` *before* the child process is
spawned. The log line includes:

- Whether the script is a root or per-package script.
- The package name and version (for per-package scripts).
- The hook name (`preinstall` / `install` / etc.).
- The resolved script body (the literal command line that will be
  passed to the shell).
- The `cwd` and prepended `PATH` entries.

Example:

```
INFO guroku::lifecycle: running postinstall for sharp@0.33.2
  cwd=.guroku/abc.../node_modules/sharp
  cmd="node install/check"
  path_prepend=[".guroku/.../sharp/node_modules/.bin", "node_modules/.bin"]
```

Failures additionally produce a `warn!` line as described in
section 5. The full stdout/stderr capture is attached to the
install report and is shown in the final summary if `--verbose` is
passed.

This logging is intentionally verbose: lifecycle scripts are the
single most common source of "why did my install do *that*?"
debugging requests, and the answer is almost always visible in
the trace.

---

## 10. Security recap

Lifecycle scripts execute arbitrary code from third-party packages
on the user's machine, with the user's privileges. This is the
same threat model npm and pnpm operate under, and guroku makes the
same trade-off (run-by-default) for ecosystem compatibility.

For the full discussion --- including the planned sandboxing work,
the allow/deny-list design, and the rationale for keeping scripts
on by default --- see
[security-model.md](./security-model.md).

If you are installing untrusted code, the safest invocation today
is:

```
guroku install --ignore-scripts
```

and then auditing or running the scripts manually as needed.
