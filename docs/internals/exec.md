# `guroku exec`

Internals doc for the `exec` subcommand in guroku v0.4.

## What it does

```
guroku exec <cmd> [args...]
```

`guroku exec` runs a command, looking it up in two places, in order:

1. `<cwd>/node_modules/.bin/<cmd>` — the project-local bin directory populated
   during `guroku install`.
2. `PATH` — the user's environment `PATH`, searched left-to-right with the
   platform's normal rules (including `PATHEXT` on Windows).

Once a binary is found, guroku spawns it as a child process. stdin, stdout,
and stderr are inherited from the guroku process. When the child exits,
guroku exits with the child's status code (or `1` if the child was terminated
by a signal and we're on Unix).

There is no shell, no expansion, no globbing. Arguments after `<cmd>` are
passed verbatim to the child.

## Why this is useful

A typical Node.js project installs CLIs as dependencies — `tsc`, `eslint`,
`vitest`, `prettier`, and so on. Those binaries land in
`node_modules/.bin/`, which is *not* on the user's `PATH`. To run them
directly, the user has to either:

- Type `node_modules/.bin/tsc --noEmit` (long, ugly, easy to typo).
- Define an npm `"scripts"` entry and run `guroku run typecheck`.
- Activate a shell helper like `direnv` or `npm-run-path`.

`guroku exec tsc --noEmit` is the short form. It is the same idea as
`pnpm exec` and `yarn exec`. It is intentionally slimmer than `npx`:
`npx` will reach out to the registry and install the package on demand if
it's missing — `guroku exec` will not (see next section).

```sh
# Run the project's own tsc, regardless of what's on PATH.
guroku exec tsc --noEmit

# Run vitest with extra flags.
guroku exec vitest --run --reporter=dot

# Pipe stdin into a project-local tool.
echo '{"a":1}' | guroku exec jq .a
```

## What it does NOT do (`dlx`)

npm's `npx` and pnpm's `dlx` will, when the requested package is not
already installed, fetch it from the registry into a cache directory and
run it from there. This is convenient but it is also a separate concern
from "run a thing I already have."

guroku v0.4's `exec` does **not** do that. If a binary is not in
`node_modules/.bin/` and not on `PATH`, `exec` returns `BinNotFound`. The
user is expected to install the package separately (`guroku add <pkg>`,
`guroku install`, or a global install) before invoking it.

A separate `guroku dlx <pkg>` command is planned and will handle the
"resolve, download, run, optionally throw away" workflow. It is not
in v0.4. Keeping these two commands separate avoids a class of footguns
where a typo in `guroku exec` would silently trigger a network install of
a typosquatted package.

## PATH propagation

Many Node CLIs themselves shell out to other Node CLIs. For example,
`vitest` may invoke `tsc`, or a build script may call `eslint --fix`
which calls `prettier`. Those nested invocations rely on the *child's*
`PATH` containing `node_modules/.bin/`.

guroku handles this by prepending `<cwd>/node_modules/.bin/` to the
child's `PATH` environment variable before spawning. The separator is
`:` on Unix and `;` on Windows. The original `PATH` is preserved after
the prefix.

```text
Parent PATH:    /usr/local/bin:/usr/bin
Child  PATH:    <cwd>/node_modules/.bin:/usr/local/bin:/usr/bin
```

This matches the behaviour of npm and pnpm. Tools written assuming this
contract — which is most of them — will keep working.

If `node_modules/.bin/` does not exist (e.g. the user ran
`guroku exec` outside any project), guroku still attempts the lookup but
the prefix is omitted from the child's `PATH`. There's no point pointing
the child at a missing directory.

## Lookup precedence

`.bin/` first, `PATH` second. This matches `pnpm exec` and `yarn exec`.

The reasoning: a project-local `tsc` should override a globally-installed
one. The whole point of pinning a tool as a dev dependency is to fix
its version for the project. If the global `tsc` won the lookup, you
could ship a build that passed locally and broke in CI (or vice-versa)
purely because of which version happened to be on someone's `PATH`.

`PATH` is the fallback so that commands like `guroku exec ls` or
`guroku exec git status` still work — useful for scripts and Makefiles
that want a single `guroku exec ...` prefix without caring whether the
target is project-local or system-wide.

## Failure modes

The `exec` command can fail in three distinct ways. Each maps to a
specific error variant in `crate::error::ExecError`.

### `BinNotFound { name }`

The command was not found in `<cwd>/node_modules/.bin/<cmd>` and not on
`PATH`. Exit code: `127` (matching POSIX shell convention for
"command not found").

```text
guroku exec eslint
error: bin not found: `eslint`
hint: did you forget `guroku install`? or `guroku add -D eslint`?
```

### `ScriptFailed { script, status }`

The child was found and spawned, but exited with a non-zero status.
guroku exits with the same status. No additional message is printed —
the child has already had its chance to write to stderr.

```text
guroku exec tsc --noEmit
src/foo.ts:12:3 - error TS2322: Type 'string' is not assignable to type 'number'.
# (guroku exits 2, same as tsc)
```

### `ScriptSpawnFailed { script, detail }`

The binary was found on disk, but the OS refused to spawn it for a
reason other than `ENOENT`. Common causes: the file is not executable
(missing `+x` on Unix), it's a binary for a different architecture, or
it failed exec-time setup. The `detail` field carries the underlying
`io::Error`.

```text
guroku exec my-cli
error: failed to spawn `node_modules/.bin/my-cli`: Permission denied (os error 13)
```

`ENOENT` at spawn time is treated as `BinNotFound` rather than
`ScriptSpawnFailed`, since it almost always means a `.bin` symlink that
points at something which has since been deleted — same user-facing
remedy as a missing bin.

## Implementation

The whole command lives in `src/commands/exec.rs`, in roughly 30 lines:

```rust
pub fn run(cwd: &Path, cmd: &str, args: &[String]) -> Result<i32, ExecError> {
    let bin = resolve_bin(cwd, cmd)?;
    let new_path = prepend_bin_to_path(cwd);

    let status = Command::new(&bin)
        .args(args)
        .env("PATH", new_path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => ExecError::BinNotFound { name: cmd.into() },
            _ => ExecError::ScriptSpawnFailed {
                script: cmd.into(),
                detail: e.to_string(),
            },
        })?;

    match status.code() {
        Some(0) => Ok(0),
        Some(code) => Err(ExecError::ScriptFailed { script: cmd.into(), status: code }),
        None => Err(ExecError::ScriptFailed { script: cmd.into(), status: 1 }),
    }
}
```

`resolve_bin` checks `<cwd>/node_modules/.bin/<cmd>` (with `.cmd` /
`.exe` suffixes on Windows) and falls back to `which::which`.

`prepend_bin_to_path` reads `std::env::var_os("PATH")`, prepends
`<cwd>/node_modules/.bin` using `std::env::join_paths`, and returns the
resulting `OsString`.

That's the whole command.

## Why we don't use `cmd /c` / `sh -c`

A natural-looking alternative would be:

```rust
Command::new("sh").arg("-c").arg(format!("{cmd} {}", args.join(" ")))
```

We deliberately do not do this, for two reasons:

1. **Quoting.** The user already split their command into `cmd` and
   `args`. Joining them back into a single shell string requires us to
   re-quote each arg correctly for the target shell. `sh`, `bash`, and
   `cmd.exe` all have different quoting rules. Round-tripping through a
   shell is a known source of injection bugs and "why did my filename
   with a space break" tickets.
2. **Semantics.** `exec` is documented as "run this binary." The user
   passed the command name as an explicit argv element. Wrapping it in a
   shell would silently enable shell features (variable expansion,
   globbing, redirection, command substitution) that the user did not
   ask for and may not expect. If the user wants a shell, they can run
   `guroku exec sh -c '...'` themselves.

## stdin

stdin is inherited, not piped or null'd. This is what makes the
following work:

```sh
echo "hello" | guroku exec my-cli
cat data.json | guroku exec jq .field
guroku exec node            # interactive REPL works
```

If we'd set `Stdio::null()` "to be safe," interactive tools (REPLs,
`vitest --watch`, `prettier --stdin-filepath`) would all silently break
or hang. If we'd set `Stdio::piped()`, we'd have to manually pump bytes
between guroku's stdin and the child, which is both more code and a
potential deadlock surface. `Stdio::inherit()` is the right answer:
the child literally gets guroku's stdin file descriptor, and the kernel
does the rest.

The same reasoning applies to stdout and stderr — both inherited, both
go straight to the user's terminal (or whatever guroku's own stdout/err
is wired up to).
