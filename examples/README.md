# Examples

This directory holds runnable example projects used to demo guroku features.
Each subdirectory is a self-contained project that exercises a specific slice
of the package manager (install flows, lockfile behavior, registry config,
workspaces, and so on). They double as smoke tests: if `guroku` can resolve
and install one of these examples cleanly, the corresponding feature is in
working order.

## Index

| Example | What it shows |
|---|---|
| [`sample-project/`](sample-project/) | Basic `guroku install` against the public npm registry. |

## Planned

- `workspace-monorepo/` (v0.4)
- `private-registry/` (v0.5)
- `lockfile-frozen/` (v0.2)

## Contributing examples

Example PRs should keep dependencies tiny and avoid network surprises. In
particular:

- No native modules (nothing that requires a C/C++ toolchain or `node-gyp`).
- No `postinstall`, `preinstall`, or other lifecycle scripts.
- Pin dependency versions so installs are reproducible.
- Prefer well-known, low-churn packages so the example does not break when
  upstream releases a new version.

The goal is for `guroku install` inside any example directory to succeed on
a clean machine with only a working network connection, and to finish in
seconds rather than minutes.
