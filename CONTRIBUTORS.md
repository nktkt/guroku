# Contributors

This file is the human-curated list of people who have contributed to guroku in
ways that aren't always reflected in `git log` — issue triage, design
discussion, security reports, and similar work that doesn't necessarily produce
a commit under the contributor's name.

For commit-level credit, the authoritative sources are:

- `git shortlog -sn` in this repository.
- The GitHub contributors graph: https://github.com/nktkt/guroku/graphs/contributors

If you've contributed and aren't listed here, please open a PR adding yourself.
We'd rather over-credit than under-credit.

## Maintainers

- **nktkt** ([@nktkt](https://github.com/nktkt)) — original author, maintainer.

## Contributors

> guroku is brand new and the contributor list will grow here. If you've
> contributed in any form (code, docs, design, security report) and would like
> to be listed, open a PR adding yourself.

## All Contributors specification

We follow the spirit of the [All Contributors](https://allcontributors.org)
specification. We don't use the emoji key or the bot — instead, we recognise
contributions in plain words. The contribution types we recognise are:

- code
- docs
- design
- ideas
- bug reports
- security
- infrastructure
- mentoring

When adding yourself (or someone else) to the contributor list, note which of
these types apply. A single person can be credited under multiple types.

## Acknowledgements

guroku's design owes a great deal to the package managers and tools that came
before it. In particular, we want to thank:

- **pnpm** — for proving that content-addressed storage and a strict, symlinked
  `node_modules` layout is not only possible but pleasant to use.
- **bun** — for showing how fast a JavaScript-ecosystem package manager can be
  when you take performance seriously from day one.
- **npm** — for the registry, the lockfile format conventions, and decades of
  hard-won lessons about what package management at scale actually looks like.
- **deno** — for rethinking module resolution, permissions, and the role of a
  runtime's package manager from first principles.
- **uv** — for demonstrating that a Rust-based package manager for a
  dynamically-typed ecosystem can be both ergonomic and dramatically faster
  than its incumbents.
- **cargo** — for the workspace model, the lockfile discipline, and the
  general standard of polish we aspire to.
- **the PubGrub paper** — Natalie Weizenbaum's "PubGrub: Next-Generation
  Version Solving" gave us a version resolution algorithm that produces
  high-quality error messages and terminates predictably. guroku's resolver
  builds directly on this work.

Any good ideas in guroku are almost certainly borrowed from one of the above.
The mistakes are our own.
