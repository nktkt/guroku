# Scripts

How guroku v0.4 runs `package.json` scripts and lifecycle hooks.

This document describes the internals of script execution: which entry
points exist, how the shell is selected, how `PATH` is augmented, how
arguments are quoted, what failure looks like, and which lifecycle
hooks the installer fires.

## Entry points

All script execution funnels through `src/scripts.rs`. Two functions
are exposed:

```rust
pub fn run_in(
    cwd: &Path,
    name: &str,
    body: &str,
    bin_dirs: &[PathBuf],
) -> Result<(), GurokuError>;

pub fn run_in_with_args(
    cwd: &Path,
    name: &str,
    body: &str,
    bin_dirs: &[PathBuf],
    args: &[String],
) -> Result<(), GurokuError>;
```

`run_in` is what `commands/install.rs` calls for lifecycle hooks; the
body is taken verbatim from the package's `scripts` map.

`run_in_with_args` exists for the `guroku run <name> -- <args...>`
shape, where extra positional args after `--` need to be appended to
the script body. The two functions are otherwise identical: same shell
selection, same `PATH` handling, same stdio inheritance, same error
type.

## Shell choice

Scripts are executed via the system shell. The selection is unconditional:

- Unix: `sh -c <body>`
- Windows: `cmd /c <body>`

```rust
#[cfg(unix)]
let mut cmd = Command::new("sh");
#[cfg(unix)]
cmd.arg("-c").arg(&full_body);

#[cfg(windows)]
let mut cmd = Command::new("cmd");
#[cfg(windows)]
cmd.arg("/c").arg(&full_body);
```

We deliberately do not probe for `bash`, `zsh`, or PowerShell. `sh` is
universally available on POSIX systems (it is required by the LSB and
by every supported Unix), and `cmd.exe` ships with every supported
Windows release. Picking a more featureful shell would mean different
behaviour on different machines for the same `package.json`, which is
exactly the bug we are trying to avoid: scripts in a published package
must run the same way on every developer's box.

This matches npm's `script-shell` default of `sh` on Unix and `cmd` on
Windows. Users who want bashisms can write `bash -c '...'` inside the
script body itself.

## PATH augmentation

Every entry in `bin_dirs` is prepended to the inherited `PATH`, in
order. The first entry has the highest priority.

```rust
let mut path = bin_dirs.iter().cloned().collect::<Vec<_>>();
if let Some(existing) = std::env::var_os("PATH") {
    path.extend(std::env::split_paths(&existing));
}
let joined = std::env::join_paths(path)?;
cmd.env("PATH", joined);
```

The install pipeline always provides two paths, in this order:

1. `<project>/node_modules/.bin` — the project's own bin directory,
   so a script can call any directly-installed dependency by name.
2. `<package>/node_modules/.bin` — the per-package bin directory,
   relevant when a package's own lifecycle script wants to invoke
   one of its own dependencies' binaries.

The current `PATH` is appended after these so the user's environment
still works (compilers, system tools, etc.) but our `.bin` shims win
on a name conflict. This is the same precedence rule npm and pnpm
use.

## Script argument quoting

`run_in_with_args` shell-quotes each user-supplied arg via the
internal `shell_quote` helper, then appends the quoted args to the
body separated by spaces.

```rust
let mut full_body = body.to_string();
for a in args {
    full_body.push(' ');
    full_body.push_str(&shell_quote(a));
}
```

### Unix quoting

On Unix, `shell_quote` checks each char against an allowlist of
alphanumerics plus a few punctuation characters that are safe inside
an unquoted shell word (`_`, `-`, `.`, `/`, `=`, `:`, `,`, `+`, `@`,
`%`). If every char is on the allowlist, the arg is emitted as-is. If
any char is not on the allowlist, the whole arg is wrapped in single
quotes, with each embedded `'` rewritten as `'\''` (close-quote,
escaped single quote, reopen-quote).

```rust
#[cfg(unix)]
fn shell_quote(s: &str) -> String {
    fn safe(c: char) -> bool {
        c.is_ascii_alphanumeric()
            || matches!(c, '_' | '-' | '.' | '/' | '=' | ':' | ',' | '+' | '@' | '%')
    }
    if !s.is_empty() && s.chars().all(safe) {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        if c == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    out
}
```

This handles spaces, glob metacharacters, `$`, backticks, semicolons,
newlines, and other shell-special bytes correctly.

### Windows quoting

On Windows, `cmd.exe` parses arguments very differently. `shell_quote`
double-quotes the arg, escapes embedded `"` as `\"`, and prefixes the
two `cmd`-meta characters `%` and `^` with `^`.

```rust
#[cfg(windows)]
fn shell_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '%' | '^' => { out.push('^'); out.push(c); }
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}
```

`%` is escaped to prevent variable expansion (`%PATH%`) and `^` is the
`cmd` escape character itself.

## Failure semantics

A non-zero exit status returns:

```rust
GurokuError::ScriptFailed {
    script: String,   // the script name, e.g. "postinstall"
    status: ExitStatus,
}
```

Spawn failures (the shell binary itself could not be launched, e.g.
`ENOENT` on `sh` because `/bin/sh` is missing in some weird container)
return:

```rust
GurokuError::ScriptSpawnFailed {
    script: String,
    source: std::io::Error,
}
```

The script name is included purely for diagnostics. It is not used to
recover or retry — the installer either propagates the error (root
scripts) or logs and continues (per-package scripts; see below).

## stdio inheritance

guroku does not capture script output. stdin, stdout, and stderr are
inherited from the parent process:

```rust
cmd.stdin(Stdio::inherit())
   .stdout(Stdio::inherit())
   .stderr(Stdio::inherit());
```

Script output goes straight to the user's terminal. There is no
buffering, no log file, no per-script section. This matches the
behaviour of npm and pnpm: builds that print progress bars, prompts,
or interactive errors all work without special handling. The
trade-off is that interleaved output from concurrent installs can look
messy; we accept that, since serializing script output would mean
hiding it until the script finishes, which is much worse for long
native builds.

## Lifecycle hooks (`commands/install.rs`)

The installer fires the following lifecycle scripts, in order:

### Root project

1. `preinstall` — before any resolution.
2. `install` — after files are linked into `node_modules`.
3. `postinstall` — after `install`.
4. `prepare` — after `postinstall`.

`prepare` running after `postinstall` matches npm. It is conceptually
a "post-postinstall" hook, used by tooling like Husky.

### Per dependency

For every package the installer materialised in this run:

1. `preinstall`
2. `install`
3. `postinstall`

Per-package scripts run with `cwd` set to the package's own directory
in `node_modules` and with `bin_dirs` set to the project's `.bin`
followed by the package's own `.bin`.

## Per-package failure policy

Per-package lifecycle scripts are best-effort. If a dependency's
`postinstall` (or any of its hooks) fails, the installer logs a
warning and continues:

```rust
if let Err(e) = scripts::run_in(&pkg_dir, "postinstall", body, &bin_dirs) {
    tracing::warn!(
        package = %pkg.name,
        version = %pkg.version,
        error = %e,
        "postinstall failed; continuing"
    );
}
```

Root-level script failures still abort the install with a non-zero
exit code.

The `--ignore-scripts` flag short-circuits all lifecycle execution
(both root and per-package) at the top of the install pipeline.

### Why best-effort for deps

The decision is pragmatic. The most common failing per-package
postinstall scripts are platform-specific:

- `fsevents` only builds on macOS; its `install` fails on Linux and
  Windows. It is a transitive dep of countless tools, including
  `chokidar`, which is a transitive dep of countless build tools.
- Native Node addons that fall back to source build when the prebuilt
  binary is missing — a missing C toolchain is common in CI images.
- Optional postinstalls that fetch network resources and time out.

Aborting the install in these cases would break standard development
workflows for no benefit; the package still installs correctly, it
just does not get its optional native acceleration. npm and pnpm
behave the same way for the same reason.

Root scripts are different: they were written by the user for this
specific project, on this specific machine, and a failure means
something the user explicitly asked for did not happen. Failing loud
is correct.

## Security model

Lifecycle scripts execute arbitrary code with the privileges of the
user running `guroku install`.

This is a real attack surface. A compromised package on the registry
can ship a malicious `postinstall` that exfiltrates environment
variables, drops persistence, or installs a cryptominer — and it
will run the moment the user adds the package to their dependencies.
This is true of npm, pnpm, yarn, and guroku alike; it is the single
largest weakness of the npm ecosystem's install model.

guroku v0.4 does not solve this. The mitigations available today are:

- `--ignore-scripts`, which skips all lifecycle hooks. This is the
  same flag npm provides, and we recommend setting it as the default
  in CI environments where you do not need native builds.
- The lockfile, which pins each dep to a specific tarball hash so a
  compromised version cannot be silently substituted on a later run.

### Future work

The intended direction is a per-package opt-in trust model:

```text
guroku trust <pkg>[@<version>]
```

Until a package is trusted, its lifecycle scripts are skipped (with
a warning). The trust list is committed alongside the lockfile. This
turns a passive attack ("I added a dep") into an active one ("I
explicitly trusted this dep's install scripts"), and lets reviewers
audit the diff.

This is not implemented in v0.4. It is tracked in the roadmap.

## Diagnostics

Setting `GUROKU_LOG=debug` causes `scripts::run_in` to log the script
body before it runs:

```shell
GUROKU_LOG=debug guroku install
# ...
# DEBUG guroku::scripts: running script name=postinstall body="node ./scripts/postinstall.js"
```

This is the recommended first step when debugging a failing
lifecycle hook. It shows exactly what guroku passed to the shell,
including any args appended by `run_in_with_args`, so you can
reproduce the failure by hand.

For deeper investigation, `GUROKU_LOG=trace` additionally logs the
final `PATH` and the `cwd` used for each spawn.
