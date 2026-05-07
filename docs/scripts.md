# Scripts

guroku supports the same `scripts` field that npm and pnpm use in
`package.json`. This page describes what you can put there, how to invoke
scripts from the CLI, and how lifecycle scripts behave during install.

If you are looking for the contributor-facing internals (how the script
runner is implemented, the env table, the IPC layer, etc.), see
`docs/internals/scripts.md` instead.

## What goes in `scripts`

The `scripts` field maps a name to a shell command. guroku reads it
verbatim and runs the value through the platform shell.

```json
{
  "scripts": {
    "build": "tsc -p .",
    "test": "vitest",
    "preinstall": "echo running preinstall",
    "postinstall": "node ./tools/postinstall.js"
  }
}
```

Names you choose yourself (`build`, `test`, `lint`, `dev`, ...) are
ordinary scripts: they only run when you explicitly invoke them. Names
from the lifecycle list (`preinstall`, `install`, `postinstall`,
`prepare`) run automatically at the appropriate point during
`guroku install`. See [Lifecycle scripts](#lifecycle-scripts) below.

## `guroku run <script>`

Runs a named script from the current package's `scripts` table.

```sh
guroku run test
```

With the `package.json` above, this invokes `vitest` in the package
root.

If the named script does not exist, guroku exits with a non-zero status
and prints the list of available scripts. If the script exists but the
underlying command fails, guroku propagates the exit code.

## Forwarding args

Anything after a `--` separator is appended to the script body before
the shell runs it.

```sh
guroku run test -- --watch
```

With `"test": "vitest"`, the command that actually executes is:

```sh
vitest --watch
```

Quoting is preserved as you would expect from a normal shell
invocation: `guroku run test -- --reporter "json verbose"` passes the
quoted string as a single argument to `vitest`.

You can also use this with build scripts, e.g.
`guroku run build -- --incremental`.

## Listing scripts

Run `guroku run` with no script name to see what is available in the
current `package.json`:

```sh
guroku run
```

Example output:

```
Available scripts in my-app:
  build         tsc -p .
  test          vitest
  preinstall    echo running preinstall
  postinstall   node ./tools/postinstall.js
```

This is the same view you get when you mistype a script name.

## Lifecycle scripts

The following script names run automatically during `guroku install`,
in this order:

1. `preinstall`
2. `install`
3. `postinstall`
4. `prepare`

You do not invoke these with `guroku run` (though you can, manually,
for debugging). They fire as part of the install pipeline once the
dependency graph has been resolved and files have been linked into
`node_modules`.

To skip every lifecycle script in a single install:

```sh
guroku install --ignore-scripts
```

This is a global switch. It applies to your own package's lifecycle
scripts and to every dependency's lifecycle scripts in the same run.
See [Security](#security) for when this matters.

## `guroku exec <cmd>`

Runs a project-local binary. Resolution order:

1. `node_modules/.bin/<cmd>` in the current package
2. `<cmd>` on the system `PATH`

This means you can install a tool as a dev dependency and use it
without writing a `scripts` entry just to wrap it:

```sh
guroku add -D eslint
guroku exec eslint .
```

The first call resolves to `./node_modules/.bin/eslint`. If you remove
ESLint as a dependency, `guroku exec eslint .` falls through to the
system `eslint` if you have one installed, otherwise it exits with
"command not found".

`guroku exec` forwards all arguments to the resolved binary unchanged.
You do not need a `--` separator here; `guroku exec` stops parsing its
own flags at the first positional argument.

## PATH inside scripts

When guroku runs anything from `scripts` (or via `guroku exec`),
`./node_modules/.bin` is prepended to `PATH` for the child process.
The implication is that scripts can refer to dependency binaries by
bare name:

```json
{
  "scripts": {
    "build": "tsc -p .",
    "lint": "eslint ."
  }
}
```

You do not need to write `./node_modules/.bin/tsc`. This matches npm
and pnpm behaviour, and it is what makes `"test": "vitest"` work even
though `vitest` is not on the user's global `PATH`.

In a workspace, the `node_modules/.bin` directory of the current
package is prepended; the workspace root's `.bin` follows.

## Shell

Scripts run through the platform shell:

- On Unix-like systems (macOS, Linux, BSDs), guroku spawns `/bin/sh`
  with `-c "<script body>"`.
- On Windows, guroku spawns `cmd.exe` with `/d /s /c "<script body>"`.

Stick to POSIX shell syntax in `package.json` if you want your scripts
to be portable. Bash-only constructs do not work under `sh` on
Debian/Ubuntu (where `/bin/sh` is `dash`) and obviously do not work
under `cmd`. In particular, avoid:

- Process substitution: `<(cmd)`, `>(cmd)`
- `[[ ... ]]` test syntax
- Arrays: `arr=(a b c)`
- `${var,,}` / `${var^^}` case conversion
- `set -o pipefail`

If you genuinely need bash, put the bash-specific code in a separate
shell script with a `#!/bin/bash` shebang and call that from your
`scripts` entry:

```json
{
  "scripts": {
    "release": "./tools/release.sh"
  }
}
```

The wrapper script can use whatever interpreter it declares.

## Per-package postinstall

When you `guroku install`, every dependency that ships its own
`postinstall` (or `preinstall` / `install`) gets to run it. This is
how packages with native build steps (e.g. things that wrap a Rust or
C library, things that download a prebuilt binary, things that write
a generated file into their own folder) get themselves into a working
state.

guroku runs each dependency's lifecycle script with that package's
own folder as the working directory and its own `node_modules/.bin`
prepended to `PATH`.

If a dependency's lifecycle script exits non-zero, guroku emits a
warning and continues. The install as a whole still succeeds. The
rationale is that failing-loudly-but-not-fatally matches what users
expect from npm and avoids one broken transitive optional native
binary blocking the whole tree. If you need stricter behaviour, run
with `--ignore-scripts` and reproduce the build steps yourself in a
controlled way.

## Security

Lifecycle scripts execute arbitrary code from the registry. A
malicious or compromised package can use its `postinstall` to read
files, exfiltrate environment variables, or run commands on your
machine. This is not a guroku-specific risk; it is inherent to any
package manager that supports lifecycle scripts.

If you do not trust every package in your dependency tree (and at
non-trivial sizes you cannot, fully), use `--ignore-scripts`:

```sh
guroku install --frozen-lockfile --ignore-scripts
```

This is the recommended hardened pattern for CI. `--frozen-lockfile`
guarantees you install exactly what `guroku.lock` describes (no
resolution drift, no surprise version bumps), and `--ignore-scripts`
guarantees no third-party code runs during install. If you have
dependencies that genuinely need a postinstall to function (e.g. they
download a native binary), allow-list them explicitly via your build
pipeline rather than re-enabling scripts wholesale.

For more on the threat model and how guroku sandboxes (and does not
sandbox) script execution, see `docs/internals/security-model.md`.

## FAQ

### Why doesn't my postinstall get an `npm_lifecycle_event` env var?

guroku v0.4 does not yet set the `npm_*` family of environment
variables (`npm_lifecycle_event`, `npm_package_name`,
`npm_package_version`, `npm_config_*`, ...). Scripts that rely on
those variables to branch on which lifecycle hook is firing will not
work yet. Setting these is planned; track the roadmap for the v0.5
line. As a workaround, write separate scripts for each lifecycle hook
or read the values from `package.json` directly inside your script.

### Can I run scripts in workspaces?

Workspace discovery already works in v0.4: `guroku workspaces` lists
the workspaces in the current root and their resolved paths.
Per-workspace script execution (`guroku run <script> --filter
<workspace>`, `guroku run <script> --recursive`) is not yet wired up.
It is scheduled for the v0.4.x line. Today, `cd` into the workspace
folder and run `guroku run <script>` from there.

### Why do my deps' postinstalls run by default?

To match npm and pnpm. Disabling them by default would mean that
packages with native build steps silently fail to set themselves up,
and users would file issues that look like guroku bugs but are
actually missing postinstalls. If you want the safer-but-stricter
behaviour, opt in with `--ignore-scripts` (see
[Security](#security)).
