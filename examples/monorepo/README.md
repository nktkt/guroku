# monorepo example

A minimal guroku workspace setup. Demonstrates the `package.json#workspaces`
field and the `guroku workspaces` discovery command introduced in v0.4.

## What this example shows

- Declaring a workspace via the `workspaces` field in the root `package.json`.
- Running `guroku workspaces` to enumerate the workspace packages guroku
  discovered from disk.

In v0.4, workspace support is **discovery-only**: guroku finds packages that
match the `workspaces` glob, parses their `package.json`, and lists them. It
does *not* yet wire up inter-workspace dependencies as local symlinks. That
auto-linking lands in v0.5.

## Layout

```
examples/monorepo/
├── package.json                                  (root, declares workspaces)
├── packages/
│   └── util/
│       └── package.json                          (@guroku-example/util)
└── README.md                                     (this file)
```

The root `package.json` declares:

```json
{
  "workspaces": ["packages/*"]
}
```

The single child package, `packages/util/package.json`, has the name
`@guroku-example/util` and version `0.1.0`.

## List workspaces

From this directory, run:

```sh
cd examples/monorepo
guroku workspaces
```

Expected output:

```
found 1 workspace package(s):
  @guroku-example/util@0.1.0  (packages/util)
```

The path on the right is relative to the workspace root (the directory
containing the root `package.json`).

## Adding more workspaces

Drop a new `package.json` under `packages/<name>/` and re-run
`guroku workspaces`. The glob `packages/*` will pick it up automatically;
no edit to the root `package.json` is required.

```sh
mkdir -p packages/another
# write packages/another/package.json with name + version
guroku workspaces
```

## What v0.4 does NOT do

- **No automatic local linking.** If you declare
  `"@guroku-example/util": "*"` in another workspace's `dependencies`,
  guroku v0.4 will try to fetch it from the npm registry (and fail with a
  404). Auto-linking to the local source tree lands in v0.5.
- **No `guroku run` fan-out.** Running a script in every workspace
  (something like `guroku run build --workspaces`) is not implemented in
  v0.4. It is on the v0.4.x backlog.
- **No workspace filtering.** There is no `--filter <pattern>` flag yet;
  `guroku workspaces` always lists every package it finds.

## What you can do today

Discovery is enough to build your own scripting glue on top. Example
shell snippet that runs a script in every workspace:

```sh
for ws in $(guroku workspaces | awk '/^  / {print $NF}' | tr -d '()'); do
  ( cd "$ws" && guroku run build )
done
```

It is awkward; this gets nicer in v0.4.x once `guroku run --workspaces`
lands. For now, scripts that only need *which packages exist and where*
work fine.

## Related docs

- `docs/workspaces.md` — user-facing guide to the `workspaces` field and
  the `guroku workspaces` command.
- `docs/internals/workspaces.md` — implementation notes: glob expansion,
  package discovery, and the planned linking phase for v0.5.
