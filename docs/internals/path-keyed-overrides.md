# Path-Keyed Overrides

Status: stable as of v1.1
Audience: guroku contributors and advanced users authoring `package.json`
overrides
Related: `overrides.md`, `resolution.md`, `dependency-graph.md`

This note describes guroku's support for **path-keyed overrides**, the
form `"a > b > c": "1.0.0"` that pins a transitive dependency only when
it is reached through a specific parent chain.

---

## 1. Why path-keyed

A flat name override like

```json
{
  "overrides": {
    "terser": "5.0.0"
  }
}
```

forces every copy of `terser` in the graph to resolve to `5.0.0`.
That is often what users want, but not always.

Consider a project that depends on both `webpack` and a hypothetical
build tool `fastify-build`, both of which have `terser` as a transitive
dependency. Suppose webpack ships a known-bad version range that fails
on a particular project, but `fastify-build` is fine with whatever it
asked for. A flat override punishes both consumers; only one needs the
intervention.

Path-keyed overrides express the narrower intent:

```json
{
  "overrides": {
    "webpack > terser": "5.0.0"
  }
}
```

This pins `terser` to `5.0.0` **only** when it is reached because
webpack pulled it in. Any other path that arrives at `terser` (for
instance through `fastify-build`) is left to normal resolution and will
get whatever `terser` range that subtree requested.

The mental model is: "in the dependency tree, when you see this chain
of edges leading to this name, force this version".

---

## 2. The format

A path-keyed override key is a string with one or more `>` separators:

```
parent > intermediate > target
```

Each segment is a package name. Names follow the usual npm rules
(scoped names like `@scope/pkg` are allowed and treated atomically).

Whitespace around `>` is tolerated. The following keys are equivalent:

```json
{
  "overrides": {
    "webpack > terser": "5.0.0",
    "webpack>terser": "5.0.0",
    "webpack  >  terser": "5.0.0"
  }
}
```

guroku trims whitespace from each segment after splitting on `>`. The
parser does **not** allow tabs or newlines inside a key; if any segment
is empty after trimming, the entire entry is rejected with a parse
error pointing at the offending key.

A key without any `>` is just a flat-name override and is processed by
the existing flat-name path; this document is concerned only with keys
that contain at least one `>`.

---

## 3. Match semantics

The keyed path must appear as a **contiguous suffix** of the resolution
path that the resolver is currently considering.

The resolution path is the chain of package names from the project root
down to the dependency being resolved. For example, when resolving
`terser` because `webpack` listed it as a dependency, and `webpack` was
listed by the root project, the path is:

```
["root", "webpack", "terser"]
```

Given a key `"webpack > terser"`, the matcher splits it into
`["webpack", "terser"]` and asks: does this sequence equal the last
two entries of the resolution path? If yes, it matches.

Three worked examples:

- Path `["root", "webpack", "terser"]`, key `"webpack > terser"`
  -> match. The last two entries equal `["webpack", "terser"]`.

- Path `["root", "webpack", "fastify", "terser"]`, key
  `"webpack > terser"`
  -> no match. The last two entries are `["fastify", "terser"]`. The
  match must be contiguous; `fastify` sits between `webpack` and
  `terser` and breaks the chain.

- Path `["root", "fastify", "webpack", "terser"]`, key
  `"webpack > terser"`
  -> match. The last two entries are `["webpack", "terser"]`. The
  prefix of the path is irrelevant; only the suffix is required to
  match.

The "contiguous suffix" rule has two consequences worth highlighting:

1. The override applies regardless of how the chain was reached,
   provided the final segment of edges matches.
2. There is no implicit wildcard. Users who want to express "any path
   that contains webpack -> terser somewhere" cannot do so with a
   single key in v1.1 (see section 7).

---

## 4. Implementation

The matcher lives in `src/overrides.rs` as `match_path`:

```rust
pub fn match_path(
    entries: &[(String, String)],
    path: &[String],
) -> Option<String> {
    for (key, version) in entries {
        let segments: Vec<&str> = key
            .split('>')
            .map(|s| s.trim())
            .collect();
        if segments.len() > path.len() {
            continue;
        }
        let tail = &path[path.len() - segments.len()..];
        if tail.iter().zip(&segments).all(|(p, s)| p == s) {
            return Some(version.clone());
        }
    }
    None
}
```

Notes on this implementation:

- `entries` is a flat list of `(key, version)` pairs, exactly as parsed
  from the user's `overrides` object. Iteration order is the
  declaration order from `package.json` (preserved by the JSON parser);
  the first matching entry wins.
- The split-trim-compare loop is allocation-light; the only allocation
  per entry is the `Vec<&str>` of segment slices. For typical projects
  the number of entries is in the low tens and key lengths are below
  ten segments, so the cost is negligible.
- Cost is linear in `entries.len() * max_segments`. There is no
  acceleration structure; benchmarks indicated that one isn't worth
  the complexity at realistic graph sizes.
- A future optimization could group entries by their final segment
  (the target name being resolved) and skip groups whose final segment
  doesn't match. The hook for that lives in the same module.

The function is pure and side-effect free; tests in
`src/overrides.rs` exercise the three example paths above and a few
edge cases (empty path, single-segment key, scoped names).

---

## 5. Precedence vs flat / glob

`lookup_with_path(name, path)` is the single entry point used by the
resolver to ask "should this (name, path) be overridden, and if so to
what version?". It walks five sources in this fixed order and returns
the first hit:

1. **Exact-path in `overrides`** -- a path-keyed entry from the
   project's `overrides` field whose key matches as described in
   section 3.
2. **Flat-name in `overrides`** -- a single-segment entry from
   `overrides` whose key equals the package name being resolved.
3. **Exact-path in `resolutions`** -- the same path-key form, but
   appearing under the yarn-style `resolutions` field.
4. **Flat-name in `resolutions`** -- a single-segment `resolutions`
   entry whose key equals the package name.
5. **Glob `**/<name>` in `resolutions`** -- yarn-classic style, where
   `**/foo` means "any foo in the graph".

Two important observations:

- **Path beats flat within the same source.** If a user writes both
  `"webpack > terser": "5.0.0"` and `"terser": "5.1.0"` under
  `overrides`, the path-keyed one wins for matching paths and the
  flat-name one wins everywhere else. This is the intuitive ordering;
  more specific intent overrides less specific intent.
- **Flat overrides beat path resolutions.** If a user has a flat
  `terser` in `overrides` and a path-keyed `webpack > terser` in
  `resolutions`, the flat `overrides` entry wins. This follows from
  the source ranking (overrides outranks resolutions globally) and
  preserves npm's behavior where `overrides` is the canonical field
  and `resolutions` is read for compatibility.

The full ordering is therefore: source rank dominates specificity
within a source. Once we are inside a source, more specific (path)
beats less specific (flat).

A glob like `**/terser` from `resolutions` is the lowest-priority
source. It is functionally similar to a flat-name entry but is only
honored when no other rule has fired.

---

## 6. Resolver integration

The resolver lives in `src/resolver.rs` and processes a queue of
"requests" -- each request is a `(name, version_spec, parent_chain)`
triple. For each request, before classifying the spec (registry, git,
file, workspace, etc.), it asks the override system:

```rust
let path = build_path(&parent_chain, name);
if let Some(forced) = overrides::lookup_with_path(name, &path) {
    spec = forced;
}
classify_and_fetch(name, &spec)?;
```

`build_path` is the obvious helper: it concatenates the parent chain
of names with the current name to produce the resolution path. The
parent chain is maintained on the queue itself; each enqueued
dependency carries the chain of its ancestors so the resolver always
has the full path available without a separate lookup.

The function name in the source is
`resolver::resolve_with_manifest_overrides`. This is the single
chokepoint for all override application; nothing else in the resolver
inspects override tables, which simplifies reasoning about precedence
and makes the override layer easy to disable for tests.

If `lookup_with_path` returns `None`, the spec is left untouched and
classification proceeds as if no override existed. If it returns
`Some(version)`, the version replaces the original spec entirely; the
resolver does not attempt to intersect the forced version with the
original range. (This matches npm's behavior. A range intersection
mode was considered and rejected for v1.1; users who want an
intersection can author the override more carefully.)

---

## 7. What v1.1 doesn't do

The path parser is intentionally minimal. The following forms are
**not** supported in v1.1 and are rejected at manifest parse time:

- **Wildcards within a path**, like `"a > * > b"`. The intent is "any
  intermediate package between a and b", but the matcher has no
  wildcard logic and the parser refuses `*` as a segment.
- **Negative patterns**, like `"!webpack > terser"` meaning
  "terser, but not when reached through webpack". There is no syntax
  reserved for this.
- **Multiple alternative paths in a single key**, like
  `"a|b > c"` meaning "match if either `a > c` or `b > c`". This is
  rejected; users wanting the same effect must write two entries.

The parser refuses anything fancier than `>`-separated bare names
(plus optional whitespace). All of the above are listed in the
backlog and may appear in a future minor release. A design note in
`docs/internals/overrides.md` discusses the tradeoffs; the short
version is that npm itself does not support these forms, and adding
them unilaterally would harm portability of `package.json` files.

If you need negative or alternation semantics today, use multiple
flat or path-keyed entries to achieve the effect, or post-process the
graph with a custom build step.

---

## 8. Comparison with npm

npm 8 and later support the same `overrides` field with the same
path-keyed format and the same contiguous-suffix matching semantics.
We have deliberately matched npm here so that a `package.json`
authored against npm continues to work under guroku without
modification, and vice versa.

Differences worth noting:

- npm allows nested object syntax for path-keyed overrides as an
  alternative to the dotted form:

  ```json
  {
    "overrides": {
      "webpack": {
        "terser": "5.0.0"
      }
    }
  }
  ```

  guroku reads this form too and normalizes it internally to the
  flat dotted form before feeding it to `match_path`. Both styles
  produce identical matching behavior; the nested form is just easier
  to write for deeply scoped overrides.

- npm has a special `"."` key that means "the current package itself";
  guroku honors this when normalizing nested forms.

- npm's documentation is occasionally ambiguous about precedence
  between path-keyed and flat entries within the same `overrides`
  object. guroku's behavior (path beats flat within the same source)
  matches what npm actually does in practice as of npm 10.

For everything else, treat npm and guroku as compatible. A regression
in this area should be reported as a bug.

---

## 9. Comparison with yarn

Yarn classic uses a `resolutions` field rather than `overrides`, and
its keys are glob patterns rather than path expressions. The most
common pattern is `**/<name>`, which matches any occurrence of `name`
anywhere in the graph -- functionally equivalent to a flat-name
override.

Yarn classic does **not** support npm-style path-keyed entries inside
`resolutions`. The `>` separator is not part of yarn's documented
syntax. guroku reads both fields, but treats path-keyed entries under
`resolutions` as a guroku/npm extension; they will be ignored by yarn
classic but honored by guroku.

The practical implication for users migrating projects between tools:

- If you author `overrides` (npm-style), guroku and modern npm honor
  it identically. Yarn classic ignores `overrides`.
- If you author `resolutions` with `**/<name>` (yarn-style), all
  three honor it.
- If you author `resolutions` with path-keyed entries, only guroku
  honors the path part; yarn falls back to flat-name behavior on the
  same key (which often does the right thing by accident).

The recommended portable form is `overrides` for new projects and
`resolutions` with `**/` globs for compatibility with yarn classic.

---

## 10. Diagnostics

Two facilities help users verify that path-keyed overrides are firing
the way they expect.

**Debug logging.** Setting `GUROKU_LOG=debug` causes the resolver to
emit one log line per applied override:

```
[debug] override matched: name=terser path=root>webpack>terser key="webpack > terser" -> 5.0.0
```

The `path` field shows the full resolution path that triggered the
match; the `key` field shows the user-authored override entry. This
is the fastest way to confirm that a particular override is firing
for a particular subtree, and equally fast to confirm that it is
**not** firing when the user expected it to.

If you suspect a precedence issue, the same log records the source
(`overrides` vs `resolutions`) and form (path vs flat vs glob) that
matched. The order of emission is the order in which the resolver
encountered the dependency; for a deterministic full listing, run
`guroku install --frozen-lockfile` against an existing lock so the
graph traversal order is stable.

**Lockfile inspection.** `guroku.lock` records the **post-override**
versions only. If you write `"webpack > terser": "5.0.0"` and run
`guroku install`, the lockfile entry for that path's `terser`
records `5.0.0` and integrity for the 5.0.0 tarball. There is no
record in the lockfile that an override was responsible -- the lock
captures the resolved graph, not the rules that produced it.

This means two things:

- A mismatch between `package.json`'s `overrides` and the lockfile's
  resolved versions almost always indicates a stale lockfile. Re-run
  `guroku install` (without `--frozen-lockfile`) to refresh.
- Consumers of the lockfile (CI, audit tooling) can ignore the
  override layer entirely; the lockfile is the source of truth for
  what is actually installed.

For deeper investigation, `guroku why <name>` walks the lockfile and
prints the resolution path(s) for any installed package. Combine with
`GUROKU_LOG=debug` to see both the rule that fired and the path it
matched against.

---

## See also

- `docs/internals/overrides.md` -- general overrides design, including
  flat-name and glob forms.
- `docs/internals/resolution.md` -- the resolver's overall structure
  and the queue that maintains parent chains.
- `docs/internals/lockfile.md` -- the on-disk format of `guroku.lock`
  and how resolved versions are stored.
- `src/overrides.rs` -- `match_path`, `lookup_with_path`, and the
  parser for path-keyed entries.
- `src/resolver.rs::resolve_with_manifest_overrides` -- the integration
  point where overrides are applied during resolution.
