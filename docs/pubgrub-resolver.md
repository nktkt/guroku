# The pubgrub resolver

guroku 1.2 ships a PubGrub-based dependency resolver as the default. This page is the shortest path between "what just happened with my install?" and the right next step.

## The TL;DR

- `guroku install` uses pubgrub by default (since v1.2).
- pubgrub is the algorithm npm's resolver was rewritten around in 2020. It produces minimal, explainable conflict reports and resolves diamond+cascade scenarios that the v1.1 BFS resolver couldn't.
- If pubgrub fails: the error message is the derivation report. Read bottom-up. Add an `overrides` entry to fix.
- Escape hatch: `GUROKU_RESOLVER=bfs guroku install` falls back to the v1.1 path.

## What pubgrub does that BFS doesn't

```
root --> lib-a@^1 --> core@^2
root --> lib-b@^1 --> core@^2 <2.5
```

If `core` has versions `2.0.0`, `2.4.0`, `2.5.0`:
- BFS sticky-first: picks `core@2.5.0` for lib-a, then chokes when lib-b wants `<2.5`. v1.1's single-step backtrack tries to find a `core` version satisfying both — `core@2.4.0` works, so v1.1 gets it right HERE.
- Cascade case: if lib-a@1.2 ALSO requires `core@^2.5` specifically (not just `^2`), v1.1 can't backtrack lib-a too. pubgrub does: it picks `lib-a@1.1.0` instead, which only requires `core@^2`, and resolves cleanly.

## Reading a pubgrub conflict

```
$ guroku install
Error: version conflict for `<resolver>`: ...
Because lib-a@1.2.3 depends on core@>=2.5
and lib-b@1.0.0 depends on core@<2.5,
lib-a@1.2.3 and lib-b@1.0.0 are incompatible.
And because root depends on lib-a@^1 and root depends on lib-b@^1,
no version solves.
```

Bottom-up:
1. **No version solves** — there is no resolution.
2. **Because root depends on lib-a@^1 and root depends on lib-b@^1** — the root's two requirements are both involved.
3. **lib-a@1.2.3 and lib-b@1.0.0 are incompatible** — a specific pair of versions can't coexist.
4. **Because lib-a depends on core@>=2.5 and lib-b depends on core@<2.5** — the leaf incompatibility.

## Fixing it

- **Add an `overrides`** in your `package.json` to pin `core` to a version that satisfies both:
  ```json
  { "overrides": { "core": "2.5.0" } }
  ```
- **Loosen the constraint**: bump or downgrade lib-a/lib-b to versions whose `core` requirements overlap.
- **Remove the conflict source**: drop one of the two roots if the project doesn't actually need both.

## Performance notes

- Pubgrub prefetches metadata for every package transitively reachable from your roots before solving. On cold caches this is more network than the v1.1 BFS resolver did.
- The HTTP/ETag cache (`http_cache.rs`) absorbs most of that on warm runs.
- Resolution itself is sub-second for typical projects. Network is the dominant cost.

## When to use the BFS escape hatch

- A pubgrub bug. (File an issue using `pubgrub_resolution_failure.yml` first.)
- Cold-cache install on a slow network where prefetching all transitives is too much overhead.
- Comparing lockfile output between resolvers.

```sh
GUROKU_RESOLVER=bfs guroku install
```

## What pubgrub doesn't change

- Lockfile schema. v1.0/v1.1 lockfiles are read by v1.2 unchanged.
- CLI surface. Same subcommands, same flags.
- `guroku::prelude` items. Same shapes.
- Resolved/Resolution types. Same fields.

## Related

- `docs/v1.2-release-notes.md` — narrative release notes.
- `docs/migration/v1.1-to-v1.2.md` — migration guide.
- `docs/internals/pubgrub-integration.md` — the implementation deep-dive.
