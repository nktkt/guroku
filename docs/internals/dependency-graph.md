# The Dependency Graph

This document explains the mental model guroku uses to think about
dependencies. It is not a description of any single data structure in
the code; rather, it is the *picture* you should hold in your head
when reading the resolver, the lockfile, and the linker.

## 1. The graph in your manifest

Your project's `package.json` declares a set of *root edges*: every
entry in `dependencies` and `devDependencies` is one edge from the
project to a named package.

Each downloaded package's own `package.json` declares more edges of
the same kind: from that package to the things it depends on.

Together, these edges form a directed graph rooted at your project.

```
                  [ your project ]
                   /     |       \
                  v      v        v
                react   chalk    eslint
                 |        |        |
                 v        v        v
             ...deps...  ...    ...deps...
```

Every node in this graph is a *package name*. Every edge is a
*dependency relationship*. The graph is what `npm install` is
ultimately asking us to materialise on disk.

## 2. Edge labels are ranges

Edges are not bare arrows. Each edge `A -> B` carries a label: the
semver range that `A` declared for `B`. For example, an edge might
read `react -> scheduler @ ^0.23.0`.

```
   react ----- "scheduler@^0.23.0" -----> scheduler
```

The resolver's job is to assign **a single concrete version** to each
node such that *every incoming edge's label is satisfied* by that
chosen version.

If `react` says `scheduler@^0.23.0` and some other dependent says
`scheduler@^0.23.2`, both ranges must be satisfied by whatever single
version of `scheduler` we pick. In the v0.2 model, that means picking
the first range we see and then checking subsequent ranges against it
(see Section 4).

## 3. Cycles can exist (and we tolerate them)

The npm ecosystem has cyclic dependencies in the wild. The classic
example is `inflight` and `wrappy`:

```
   inflight  ----->  wrappy
       ^               |
       |               |
       +---------------+
```

`inflight` depends on `wrappy`, and historically `wrappy` depends back
on `inflight`. This is not a bug in those packages; npm permits it,
and any resolver targeting npm has to deal with it.

The v0.2 resolver handles cycles **for free** because of the
*sticky-first-choice rule*: once a name appears in the `chosen` map,
we never re-enqueue it. When the BFS reaches `inflight` for the
second time (via the back-edge from `wrappy`), it sees that
`inflight` is already chosen, and stops.

Concretely, the loop looks like this:

```
queue: [inflight]
visit inflight  -> chosen[inflight] = 1.0.6
                   enqueue (wrappy, ^1)
visit wrappy    -> chosen[wrappy]   = 1.0.2
                   enqueue (inflight, ^1)   <-- back-edge
visit inflight  -> already in chosen, skip
queue empty -> done
```

No special cycle detection code is required. The chosen-set itself
acts as the visited-set.

## 4. Diamonds and conflicts

The interesting case is a *diamond*: two paths from the root reach
the same package, possibly with different range labels.

```
        root
        / \
       A   B
        \ /
         C
```

Suppose:
- `A` declares `C@^1`
- `B` declares `C@^2`

There is no single version of `C` that satisfies both ranges. This is
a real, fundamental conflict.

v0.2's behaviour:

1. BFS visits `A` first, sees the edge `A -> C@^1`, picks (say)
   `C@1.4.0`, and writes `chosen[C] = 1.4.0`.
2. BFS later visits `B`, sees the edge `B -> C@^2`, and asks "does
   `1.4.0` satisfy `^2`?". It does not.
3. The resolver returns an error: `conflict on C: chosen 1.4.0 does
   not satisfy ^2 (required by B)`.

Whichever range the BFS sees *first* wins. The other request is
rejected. Order is essentially the order roots appear in
`package.json`, then the order each package lists its own
dependencies.

This is the spot where v0.2's algorithm is weakest. A smarter
resolver such as **PubGrub** would *backtrack*: try a different
version of `C` (or of `A`, or of `B`) until it found a globally
consistent assignment, or produce a clear unsatisfiability proof.
v0.3 will likely move in this direction.

## 5. What the resolver outputs

Although the input is a graph, the **output** of `resolve` is a flat
map:

```rust
pub struct Resolution {
    pub packages: BTreeMap<String, Resolved>,
}
```

One entry per *name*. Not per (name, version) pair. Not a graph.

The graph structure is preserved *indirectly*: each `Resolved` value
holds the package's own `package.json` (as `info`), and
`info.dependencies` is a `BTreeMap<String, String>` of name -> range.
You can reconstruct the edges by looking up each name in the outer
map.

```
resolution.packages
  "react"     -> Resolved { info: { version: "18.3.1",
                                    dependencies: { "scheduler": "^0.23.0",
                                                    "loose-envify": "^1.1.0" } } }
  "scheduler" -> Resolved { info: { version: "0.23.2",
                                    dependencies: { "loose-envify": "^1.1.0" } } }
  ...
```

Edges in the graph correspond to entries in each `info.dependencies`,
and the *targets* of those edges are the resolved versions in the
outer map.

## 6. Why flat output is OK in v0.2

The linker for v0.2 writes a flat layout:

```
node_modules/
  react/
  scheduler/
  loose-envify/
  ...
```

There is exactly one directory per package name, regardless of how
many places in the graph reference it. Different paths through the
graph that converge on the same name end up at the same place on
disk.

This is fine *because* the resolver guarantees every name has exactly
one chosen version. There is no need to keep track of "the C that A
saw" versus "the C that B saw" -- they are the same C, by
construction.

v0.3 will introduce a stricter layout that allows multiple versions
of the same name to coexist (mirroring npm's nested `node_modules`).
At that point, flat output is no longer sufficient: the linker will
need to know *which consumer wanted which version*, so that two
consumers asking for incompatible `C` ranges can each get the right
copy. The resolver will need to produce a real graph, not just a
flat map.

In short: v0.2's flat output is a deliberate simplification, valid
only because v0.2 also rejects diamond conflicts. The two design
choices belong together.

## 7. Walking the graph yourself

If you want to inspect the resolved graph from your own code, the
public API is straightforward:

```rust
use guroku::registry::RegistryClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = RegistryClient::with_default_registry()?;
    let roots = vec![("react".into(), "^18".into())];
    let resolution = guroku::resolver::resolve(&client, &roots).await?;

    for (name, r) in resolution.iter() {
        println!("{} -> {} (deps: {:?})",
                 name, r.info.version, r.info.dependencies);
    }
    Ok(())
}
```

To do a real graph walk (e.g. find the depth of each package from a
root), iterate `info.dependencies`, look each name up in
`resolution`, and recurse -- using a `visited` set, since cycles are
allowed.

## 8. Glossary

- **root** -- the project itself; the unique node with no incoming
  edges. Its outgoing edges are `dependencies` plus `devDependencies`
  in your `package.json`.
- **edge** -- a directed dependency relationship from one package to
  another, labelled with a semver range.
- **node** -- a package name. After resolution, each node is bound to
  exactly one concrete version.
- **transitive dep** -- any node reachable from the root by more than
  one edge; that is, a dependency you did not declare yourself but
  inherited from something you did declare.
- **conflict** -- a situation where two edges into the same node
  carry ranges with no version in common. v0.2 rejects these; v0.3
  may resolve some of them by backtracking or by allowing multiple
  versions of the same name.
- **sticky-first** -- the v0.2 rule that once a name has been chosen,
  the choice is final: later edges into that name are checked for
  compatibility but never trigger a re-pick. This is what makes the
  resolver simple, total on cyclic graphs, and incomplete on
  diamonds.
