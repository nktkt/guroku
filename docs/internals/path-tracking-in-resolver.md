# Path tracking in the resolver

This note explains how the resolver carries a per-edge dep-chain path through
its BFS, why we added it in v1.1, and how that path feeds the two consumers
that need it: path-keyed override lookup and conflict reporting.

## 1. Why

Two features in v1.1 need to know not just "what package am I resolving?" but
"how did I get to it?":

- **Path-keyed overrides.** A user can write
  `"overrides": { "foo": { "bar": "1.2.3" } }` to say "when bar is reached via
  foo, pin it to 1.2.3". To honor that we have to know whether the current
  resolution step reached `bar` through `foo` or some other parent. A flat
  name lookup is not sufficient.
- **Conflict errors.** When two requirements on the same package cannot be
  reconciled, the user wants to see which dep chain pulled in which range.
  "`a > b > c` wanted ^1, but `a > d` wanted ^2" is far more actionable than
  "something wanted ^1 and something else wanted ^2". The path is what makes
  that message possible.

Both features want the same data, so we plumb it through the BFS once.

## 2. The data

In v1.0 the resolver's BFS queue carried tuples of:

```
(name: String, range: String)
```

In v1.1 each queue entry is:

```
(name: String, range: String, path: Vec<String>)
```

Semantics of `path`:

- It is **the chain of names from the root to the package being processed,
  exclusive of the leaf**. The leaf is the `name` field of the same tuple.
- For root deps (deps declared directly in the consumer's `package.json`),
  `path` is empty when the entry is enqueued. The leaf name is appended to
  produce `[name]` only at lookup/error time.
- When we expand a resolved package and enqueue its children, each child's
  `path` is `parent.path + [parent.name]`.

So if the root depends on `a`, which depends on `b`, which depends on `c`, the
queue entry for `c` carries `path = ["a", "b"]` and `name = "c"`. The "full
path including leaf" `["a", "b", "c"]` is materialized on demand.

## 3. Override lookup

Override lookup happens once per dep, right after we pop a queue entry and
before we pick a version. The call is:

```
let mut full_path: Vec<&str> = path.iter().map(String::as_str).collect();
full_path.push(&name);
let pin = overrides::lookup_with_path(&manifest, &full_path);
```

Notes:

- `lookup_with_path` takes `&[&str]`, not `&[String]`. The slice-of-borrows
  form means the lookup itself never allocates a new `Vec<String>`; we build
  one `Vec<&str>` per dep and reuse the underlying `String`s in place.
- `manifest` here is the root manifest's overrides table. The function walks
  the table's matching ladder (most-specific path first, then progressively
  shorter suffixes, then bare-name) and returns the first hit.
- If nothing matches, `lookup_with_path` returns `None` and the resolver
  proceeds with the original range.

The matching ladder itself is documented in `path-keyed-overrides.md`. This
file is only concerned with how the path gets there.

## 4. Conflict surfacing

When `try_backtrack` returns `None` for a package, the resolver has a hard
conflict: there is no version of `pkg` that satisfies every accumulated
constraint. We need to build a `ResolutionConflict` describing it.

For each conflicting requirement we have the queue entry that produced it,
which means we have its `path` and `name`. We render that into a
human-readable string:

```
let chain = format_path(&path, &name); // "a > b > c"
ResolutionConflict {
    package: name.clone(),
    range: range.clone(),
    requested_by: chain,
    ...
}
```

`format_path` is the canonical formatter: it joins `path` and the leaf with
` > `. An empty path with leaf `foo` formats as `"foo"` (a root dep).

This replaces v1.0's `requested_by`, which was a flat
`format!("{name}@{range}")` and gave the user no chain context. The field is
the same name on the struct; only the contents are richer.

## 5. Why `Vec<String>` and not `Rc<...>`

The obvious efficiency concern is that every child entry clones the parent's
path, so a chain of depth N does N clones to enqueue N children, etc. We
chose `Vec<String>` anyway:

- It is dramatically simpler to print, log, and step through in a debugger.
  No interior mutability, no shared-ownership puzzles in error paths.
- We do not yet have evidence of dep trees deep enough that the cloning
  shows up in profiles. The v1.1 fixture suite tops out around depth 12.
- The optimization, when we want it, is mechanical: switch the field to
  `Rc<[String]>` (or `Arc<[String]>` if we go multi-threaded in the
  resolver) and rebuild on append. We've left a TODO referencing this note
  on the queue type.

If profile data from real registries shows path-cloning in the top-N, we'll
revisit in v1.2. Until then, clarity wins.

## 6. What's NOT in the path

The path tracks **local names**, i.e. the keys the user wrote in their
`package.json` (or that a transitive dep wrote in its own manifest). It does
**not** track aliased real_names.

Concretely: if a user writes
`"dependencies": { "lodash-fork": "npm:lodash@^4" }`, the path segment for
that edge is `"lodash-fork"`, not `"lodash"`. Aliasing is a leaf decoration
applied when we go to fetch the package; it does not change which key the
parent used to reach the dep, which is what overrides and conflict messages
care about.

This matches what users see in their own files, which is the whole point of
surfacing the path in the first place.

## 7. Testing

Two test files cover the path machinery directly:

- `tests/resolution_conflict_path_format.rs` — pins down `format_path` and
  the `requested_by` field. Covers the empty-path (root) case, single-hop,
  multi-hop, and the rendering of names that contain `>` (we don't escape;
  we document that npm package names cannot contain `>` so it's fine).
- `tests/overrides_path_keyed.rs` — drives `lookup_with_path` through the
  matching ladder: exact full-path match, suffix match, bare-name match, no
  match. Each case asserts on the chosen pin and that we did not alloc a
  fresh `Vec<String>` (verified indirectly by the `&[&str]` signature).

End-to-end coverage of "BFS produces the right path" lives in the resolver's
own integration tests; those are not specific to this feature and are not
listed here.
