# with-bin

A minimal example demonstrating how guroku v0.4 surfaces executable
entries declared by a dependency's `package.json#bin` field.

## What this example shows

- `node_modules/.bin/<name>` shims created for direct dependencies that
  declare a `bin` field in their `package.json`.
- Running those bins through `guroku exec`, which resolves the shim and
  executes it with the current shell's `PATH` augmented by
  `node_modules/.bin`.

The dependency in `package.json` is `cowsay`, which declares a `cowsay`
bin entry. After install, guroku surfaces it under
`node_modules/.bin/cowsay`.

## Try it

```sh
cd examples/with-bin
rm -rf node_modules guroku.lock
guroku install
ls -la node_modules/.bin/
```

Expected: a `cowsay` symlink in `.bin/` pointing at the real script
inside the content-addressed store.

## Run via `guroku exec`

```sh
guroku exec cowsay "hello from guroku"
```

Note that cowsay is a Node.js script — running it requires `node` on
`PATH`. `guroku exec` does not bundle a Node runtime; it only resolves
the bin and invokes it.

## Run via the symlink directly

The same effect, without going through `guroku exec`:

```sh
./node_modules/.bin/cowsay "hello"
```

This is useful for build scripts and `package.json#scripts` entries,
where `node_modules/.bin` is conventionally on `PATH`.

## Why this works

During install, guroku reads each direct dependency's `package.json`
and inspects its `bin` field. For cowsay this looks roughly like:

```json
{
  "name": "cowsay",
  "bin": { "cowsay": "./cli.js" }
}
```

For every entry, guroku creates a symlink at
`node_modules/.bin/<name>` pointing at the CAS-hardlinked file inside
the store, e.g.:

```
node_modules/.bin/cowsay
  -> ../.guroku/cowsay@<version>/node_modules/cowsay/cli.js
```

The target is itself a hardlink into the global content-addressed
store, so the bytes are shared across every project on the machine.

## Inspect the chain

```sh
readlink node_modules/.bin/cowsay
readlink -f node_modules/.bin/cowsay   # GNU; macOS uses `realpath`
```

The first command shows the immediate symlink target. The second
resolves the entire chain down to the file in the CAS.

## Caveats

- The bin script's shebang line (`#!/usr/bin/env node`) determines
  what interpreter runs. Without `node` on `PATH`, you get a fairly
  cryptic `ENOEXEC` (or, on some systems, "no such file or directory"
  pointing at `env`). Install Node separately.
- Only DIRECT dependencies get bin shims in v0.4. Transitive packages
  that ship a bin are still extracted into the store, but no shim is
  written for them under `node_modules/.bin`. If you want a transitive
  bin available, add it as a direct dependency.
- On Windows, guroku v0.4 does not yet emit `.cmd` / `.ps1` wrappers;
  this example assumes a POSIX shell.

## Related docs

- `docs/internals/bin-shims.md` — how shim creation, conflict
  resolution, and permission bits are handled.
- `docs/internals/exec.md` — what `guroku exec` does to `PATH` and how
  it locates the target binary.
