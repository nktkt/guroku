# with-scripts

A small example demonstrating how guroku v0.4 handles `package.json#scripts`,
the `guroku run` command, argument forwarding via `--`, and the lifecycle
hooks fired during `guroku install`.

## What this example shows

- How `package.json#scripts` entries are surfaced by guroku.
- Invoking arbitrary scripts with `guroku run <name>`.
- Forwarding extra arguments through to the script with `guroku run <name> -- <args...>`.
- Lifecycle hooks (`preinstall`, `postinstall`) running automatically as
  part of `guroku install`, and how to suppress them.

The package declares the following scripts:

- `preinstall` and `postinstall` (lifecycle hooks).
- `build` and `test` (typical user scripts).
- `say-hi` and `list-args` (used to demonstrate argument forwarding).

It also pulls in a single dependency, `ms`, so that `guroku install` has
real install work to do alongside the lifecycle hooks.

## Try it

```sh
cd examples/with-scripts
rm -rf node_modules guroku.lock
guroku install
```

The output should include lines such as:

```
[preinstall] starting
[postinstall] complete
```

These come from the `preinstall` and `postinstall` entries in
`package.json` and confirm that guroku is firing the lifecycle hooks
around the dependency resolution and linking phase.

## Skip lifecycle scripts

To install dependencies without running any of the lifecycle hooks, pass
`--ignore-scripts`:

```sh
guroku install --ignore-scripts
```

Notice that the `[preinstall] starting` and `[postinstall] complete`
lines are absent from the output. This is useful in CI or when you do
not trust the scripts of transitive dependencies.

## List available scripts

Running `guroku run` with no arguments prints the scripts declared in
the current `package.json`:

```sh
guroku run
```

This is the equivalent of `npm run` with no arguments.

## Run a specific script

```sh
guroku run build
guroku run test
```

These execute the `build` and `test` entries respectively. The output
will be the placeholder `echo` lines from the script bodies.

## Pass args

Anything after `--` on the command line is forwarded to the script as
positional arguments:

```sh
guroku run say-hi -- world
guroku run list-args -- a b "c d"
```

## What to look for

- The `--` separator passes through cleanly. `guroku run say-hi -- world`
  invokes `printf 'hello from %s\n' "$1"` with `$1=world`, so the output
  is `hello from world`.
- `guroku run list-args -- a b "c d"` shows that `c d` is passed as a
  single argument: the output contains three lines (`a`, `b`, `c d`),
  not four. This is preserved by guroku's shell-quoting layer, which
  reconstructs the argv before handing it to the shell.

## Failure semantics

To see how guroku reports script failures, change the body of one of
the scripts in `package.json` to `exit 7` and re-run it:

```sh
guroku run <name>
```

guroku exits with status 1 (not the underlying 7) and prints an error
describing which script failed and what its exit code was. The
non-zero exit lets shell pipelines and CI runners detect the failure
without having to parse output.

## Related docs

- `docs/scripts.md` for the user-facing description of the scripts
  feature, including the full list of supported lifecycle hooks.
- `docs/internals/scripts.md` for the implementation details: how the
  shell is selected, how argv is quoted, and how lifecycle ordering
  interacts with the install graph.
