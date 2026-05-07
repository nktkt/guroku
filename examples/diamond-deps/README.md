# diamond-deps

A small example showing how guroku v0.2 handles the classic
"diamond dependency" shape: two top-level packages that each pull
in the same transitive package via different paths.

The `package.json` here declares two top-level deps:

- `supports-color@^9`
- `ansi-styles@^6`

Both of these reach into the `color-convert` / `color-name` family
of packages, so they share at least one transitive dependency.

## What this example shows

The diamond-dependency pattern. Two top-level deps reach the same
transitive package via two different paths, and possibly with
different version ranges. A package manager has to decide:

- Do both top-level deps get the *same* copy of the shared
  transitive (deduplication), or do they each get their own?
- If the two parents disagree about which range is acceptable,
  what happens? Pick one? Install both? Error out?

guroku v0.2 takes a deliberately simple position on both of these,
described below.

## The pattern

```
        examples/diamond-deps
                / \
       supports-color  ansi-styles
                \ /
              shared transitive
```

The two top-level deps form the "top" of the diamond; the shared
transitive sits at the "bottom". The interesting question is what
shows up in `node_modules` and in `guroku.lock`.

## Try it

```sh
cd examples/diamond-deps
rm -rf node_modules guroku.lock
guroku install
ls node_modules
cat guroku.lock | head -40
```

You should see one entry per resolved package in the lockfile,
including a single entry for the shared transitive (not two), and
a `node_modules` directory laid out in the flat npm-compatible
shape.

## What v0.2 does

The resolver visits roots breadth-first. As it walks the graph it
keeps a map of `name -> resolved version`. The first version that
satisfies the first range it sees for a given package name wins,
and that becomes the chosen version for the whole graph.

If a later parent imposes a range that *also* accepts that already-
chosen version, everything is fine and the package is shared.

If a later parent imposes a range that does *not* accept the
already-chosen version, the resolver does not try to backtrack or
pick an older major of the parent. It surfaces an error of the
form:

```
version conflict for <name>: already chose <x>, but <parent> requires <range>
```

In other words, v0.2 dedupes optimistically and fails loudly. It
does not silently install two copies, and it does not silently
downgrade.

## What v0.3 will do

The planned PubGrub integration (see
`docs/internals/algorithm-notes.md`) will replace the greedy walk
with a real version-solving step. Given the same diamond, v0.3
will try to find a combination of `supports-color` and
`ansi-styles` versions whose transitive constraints are mutually
satisfiable, even if that means picking an older major of one of
the top-level deps.

The v0.2 error message above is roughly the case PubGrub will
turn into a successful resolution where one exists.

## How to cause a conflict

If you want to see the v0.2 error in action, edit `package.json`
to pin one of the top-level deps to a major that forces a
specific transitive version that the other top-level dep does
not accept. For example, pin `supports-color` to a major whose
required `color-convert` range disjoints from the range that
`ansi-styles` accepts.

We do not preconfigure this in the example, because it would
just break `guroku install` for anyone running it. The resolver
test suite covers the conflict path directly.

## Related reading

- `docs/internals/dependency-graph.md` — how guroku represents
  the dep graph internally and how dedup is implemented in v0.2.
- `docs/internals/algorithm-notes.md` — notes on the current
  greedy resolver and the planned PubGrub-based resolver for
  v0.3.
