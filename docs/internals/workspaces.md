# Workspaces

This document describes how guroku v0.4 handles npm-style workspaces:
how the `workspaces` field in `package.json` is parsed, how member packages
are discovered, and what's intentionally deferred to a later milestone.

## Scope of v0.4

v0.4 ships with **discovery only**. Concretely:

- `package.json#workspaces` is read from the project root.
- Each entry is treated as a glob; `<root>/<glob>` is expanded to a list
  of candidate directories.
- For each candidate, we read its `package.json` (if present) and
  produce a `Workspace { root, manifest }` record.
- The CLI exposes the resulting list via `guroku workspaces`, which
  prints the discovered members in alphabetical order.

What v0.4 does **not** do:

- The resolver does not treat workspace packages as first-class locals.
  A dependency on a workspace package still goes through the registry
  resolver and falls back to `404` if the name isn't published.
- The linker does not symlink or hardlink from workspace roots.
- The lockfile has no concept of "this entry came from a workspace
  member."

These integrations are scheduled for v0.5 and are described under
"v0.5 plan" below.

## The `workspaces` field

The ecosystem disagrees on the shape of this field. We accept both
forms.

### npm shape (array)

```json
{
  "name": "monorepo",
  "private": true,
  "workspaces": ["packages/*", "tools/*"]
}
```

### pnpm shape (object)

```json
{
  "name": "monorepo",
  "private": true,
  "workspaces": {
    "packages": ["packages/*", "tools/*"]
  }
}
```

Note: pnpm itself uses a separate `pnpm-workspace.yaml`, but a number of
projects in the wild stash an equivalent shape under
`package.json#workspaces` for tooling compatibility. We accept it because
it costs nothing.

### Manifest API

`Manifest::workspace_globs` normalises both into `Vec<String>`:

```rust
impl Manifest {
    pub fn workspace_globs(&self) -> Vec<String> {
        match &self.raw_workspaces {
            None => Vec::new(),
            Some(Value::Array(a)) => a
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect(),
            Some(Value::Object(o)) => o
                .get("packages")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }
}
```

Anything else (a string, a bool, etc.) yields an empty list. We do not
hard-error here — manifests in the wild are creative, and the cost of
rejecting a malformed-but-otherwise-irrelevant `workspaces` field is
higher than the value.

## `workspaces::discover(cwd)`

The discovery algorithm is intentionally small.

```text
discover(cwd) -> Result<Vec<Workspace>>:
    1. Read <cwd>/package.json.
       - If absent, return Ok(vec![]).
       - If present but unparseable, bubble ParseManifest.
    2. For each glob in manifest.workspace_globs():
       a. pattern = cwd.join(glob).to_string()
       b. Expand via glob::glob(pattern):
          - On glob-syntax error: return GurokuError::Other
            { msg: "invalid workspace glob `{glob}`: {err}" }.
       c. For each matched path:
          - If not a directory, skip.
          - If <path>/package.json is missing, skip.
          - Otherwise, read & parse it. ParseManifest errors bubble.
          - Push Workspace { root: path, manifest } onto the list.
    3. Deduplicate by canonicalised root path.
    4. Sort alphabetically by root path.
    5. Return.
```

Notes on each step:

- **Step 1** — A missing root manifest is not an error in this context.
  `guroku workspaces` should print "no workspaces" rather than failing,
  so that running it in a directory that just isn't a JS project still
  produces a sensible exit.
- **Step 2c** — Skipping non-directories matters because `glob` will
  happily match files. A pattern like `packages/*` may grab a stray
  `packages/.DS_Store`.
- **Step 2c** — Skipping directories without a `package.json` is
  deliberate. Empty scaffolds, build outputs, and other detritus often
  sit alongside real members.
- **Step 3** — The same path can match more than one glob (e.g.
  `packages/*` and `packages/web*`). Dedup keeps `guroku workspaces`
  from printing duplicates.
- **Step 4** — Determinism. Tests compare exact output; CI compares
  exact output; humans skim it. Sorting is cheap.

### Workspace struct

```rust
pub struct Workspace {
    pub root: PathBuf,
    pub manifest: Manifest,
}
```

That's deliberately the minimum. v0.5 will likely grow this — e.g. a
"name" accessor that pulls from `manifest.name` — but for now callers do
that themselves.

## v0.5 plan: workspace inter-deps

The interesting work is letting `pkg-a` depend on `pkg-b` where both
live in the same workspace, and having `guroku install` wire up the
local source directly.

Three subsystems need changes.

### Resolver

Today the resolver has a small "local" table, populated by
`file:`/`link:` specifiers. v0.5 extends this to include workspace
packages keyed by `manifest.name`. When a dependency request hits a
name in that table, the resolver short-circuits the registry lookup and
emits a `Resolved::Local { workspace_root }` node into the dependency
graph.

```text
resolve(name, range, cx):
    if let Some(ws) = cx.workspaces.get(name):
        if range.matches(ws.manifest.version):
            return Resolved::Local { root: ws.root.clone() };
        // else: fall through, with a warning
    ...registry path as before...
```

The version check is important. If a workspace declares
`pkg-b@^2.0.0` but the local `pkg-b` is at `1.4.0`, we should not
silently substitute — npm doesn't, pnpm doesn't, and doing so would
mask real bugs.

### Linker

A new "from-source" branch sits alongside the existing CAS-based one.
For a `Resolved::Local` node, the linker symlinks (or hardlinks, on
platforms where symlinks are awkward) the workspace root into
`node_modules/<name>` rather than reading from the CAS.

The implementation reuses the existing strict-layout path code; only
the source location changes. See `docs/internals/symlinks.md` and
`docs/internals/hardlinks.md` for the platform-specific bits.

### Lockfile

A new `workspace` key on lockfile entries:

```text
"pkg-b@workspace:packages/b": {
    "workspace": true,
    "version": "1.4.0",
    "dependencies": { ... }
}
```

The specifier syntax (`workspace:<relative-path>`) follows pnpm's
convention. The `workspace: true` field is redundant given the
specifier prefix, but it makes diff-reading easier and matches what
yarn berry writes.

## Why discovery alone in v0.4

The integration described above is a meaningful chunk of work — the
resolver changes alone touch the dependency-graph builder, the
conflict resolver, and the lockfile reader. Discovery, by contrast, is
self-contained and unblocks two near-term wins:

- `guroku workspaces` — a diagnostic command that has been requested
  for debugging the existing install path. People want to know what
  guroku thinks the workspace members are, regardless of whether
  inter-deps are wired up.
- `guroku run -ws <script>` — running a script in every workspace
  member, planned for v0.4.x. This needs the member list but does
  **not** need resolver or linker changes; each member's `node_modules`
  is installed independently today.

Shipping discovery first also de-risks v0.5: we get real-world
manifests through `Manifest::workspace_globs` before the resolver
starts depending on its output.

## Error handling

### Glob errors

An invalid glob pattern (e.g. unbalanced brackets) returns
`GurokuError::Other` with a message that includes the original glob
string and the underlying parse error:

```text
invalid workspace glob `packages/[`: Pattern syntax error near position 9: invalid range pattern
```

We use `Other` rather than a dedicated variant because glob errors
should be rare and the message is more useful than any code we'd
attach.

### Sub-package manifest errors

If a candidate directory has a `package.json` but it doesn't parse, the
underlying `ParseManifest` error bubbles up unchanged. The error
message already includes the path, so the user sees exactly which
member is broken.

### Missing root manifest

Returns `Ok(vec![])`. See "Step 1" above for rationale.

## Comparison with other tools

| Behaviour                              | npm | pnpm | yarn classic | yarn berry | guroku v0.4 | guroku v0.5 |
|----------------------------------------|-----|------|--------------|------------|-------------|-------------|
| Reads `package.json#workspaces`        | yes | yes  | yes          | yes        | yes         | yes         |
| Reads `pnpm-workspace.yaml`            | no  | yes  | no           | no         | no          | no          |
| Inter-deps resolved to local source    | yes | yes  | yes          | yes        | no          | yes         |
| `nohoist` field                        | yes | n/a  | yes          | n/a        | no          | no          |
| Workspace protocol (`workspace:*`)     | no  | yes  | no           | yes        | no          | yes         |

`yarn.workspaces.nohoist` is not on the roadmap. The strict layout
makes hoisting non-issues moot, and we have no plans to support a flat
layout that would re-introduce them.

## Testing strategy

Two integration tests cover discovery:

- `tests/workspaces_discover_basic.rs` — sets up a `TempDir` with a
  root manifest using the npm array form, two glob-matched member
  directories with valid `package.json` files, one decoy directory
  without a manifest, and one stray file. Asserts the output is the
  two valid members in alphabetical order.
- `tests/workspaces_object_form.rs` — same fixture, but the root
  manifest uses the pnpm object form. Asserts identical output to the
  array test, demonstrating that `workspace_globs` normalises both.

We deliberately don't unit-test `Manifest::workspace_globs` in
isolation; the integration tests cover both forms via real fixtures,
and a unit test would just duplicate that.

Future tests for v0.5 will cover:

- A workspace member depending on another workspace member by name.
- Version-mismatch fallback to the registry.
- Lockfile round-trip with `workspace:` specifiers.
- Linker hardlink/symlink selection for from-source entries.
