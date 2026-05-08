# with-file-dep

## What this example shows

This example demonstrates installing a local-source dependency via the `file:`
specifier under guroku v0.5. The `package.json` in this directory declares:

```json
"my-local-lib": "file:../local-lib"
```

guroku resolves the path relative to the consumer package, hardlinks the
source files into its strict-layout store, and symlinks them into
`node_modules/` just like any registry-sourced package.

## Setup

The sibling `local-lib/` directory is not checked in. Create it before
installing:

```sh
cd examples/with-file-dep
mkdir -p ../local-lib
cat > ../local-lib/package.json <<'JSON'
{
  "name": "my-local-lib",
  "version": "0.1.0",
  "main": "index.js"
}
JSON
echo 'module.exports = { hello: () => "from local-lib" };' > ../local-lib/index.js
```

## Install

Wipe any prior state and run a fresh install so the `file:` spec is the only
input:

```sh
rm -rf node_modules guroku.lock
guroku install
```

## Verify

The consumer should see `my-local-lib` resolved through the strict layout:

```sh
ls -la node_modules/my-local-lib/
# package.json and index.js should be present.
readlink node_modules/my-local-lib
# Resolves into node_modules/.guroku/my-local-lib@0.1.0/...
```

The symlink target lives under `node_modules/.guroku/`, and the files inside
that directory are hardlinks back to the source in `../local-lib/`.

## Live edits and hardlinks

Because guroku materializes `file:` dependencies with hardlinks, files in
`../local-lib/` share inodes with the copies under
`node_modules/.guroku/my-local-lib@0.1.0/`. Editing `index.js` in the source
directory immediately changes the bytes a consumer reads through
`node_modules/my-local-lib/index.js` -- no reinstall required.

Caveat: editors that use "atomic write" semantics (write to a tmp file, then
`rename(2)` it into place) replace the source path with a brand-new inode.
The hardlink in the store still points at the old inode, so the next read via
`node_modules` sees the OLD content. Workaround: re-run `guroku install` to
rebuild the hardlink.

## What the lockfile shows

After install, `guroku.lock` will contain an entry for the local package that
looks roughly like:

```json
"my-local-lib@0.1.0": {
  "resolved": "file:///guroku-local-source",
  ...
}
```

The `file:///guroku-local-source` URL is a placeholder. The install pipeline
never refetches it -- reproducibility for `file:` deps comes from the spec in
`package.json` (`file:../local-lib`), not from anything in the lockfile.

## Cleanup

```sh
rm -rf ../local-lib
```

## Related docs

- `docs/file-deps.md`
- `docs/internals/file-deps.md`
