# guroku

> A fast, Rust-powered package manager for the JavaScript ecosystem.

`guroku` is an experimental package manager inspired by [npm](https://www.npmjs.com/), [pnpm](https://pnpm.io/), and [bun](https://bun.sh/), built from scratch in Rust. The goal is to combine pnpm's content-addressable storage model with bun's installation speed, while staying small, hackable, and easy to read.

> **Status:** v0.2 ships an end-to-end install path with a real semver-aware resolver and a committed `guroku.lock`. Storage is still a flat copy (CAS + hardlinks land in v0.3), and lifecycle scripts / workspaces are not yet supported. Pre-alpha; expect rough edges.

---

## Why another package manager?

- **Speed.** Parallel network I/O, hardlink-based linking, and a content-addressable store on disk.
- **Correctness.** Strict, non-flat `node_modules` layout to prevent phantom dependencies.
- **Small surface area.** A focused CLI that does installs, lockfiles, and scripts well — and nothing else.
- **Readable code.** Written as a learning artifact: every module should be approachable in an afternoon.

`guroku` is **not** a JavaScript runtime. It does not bundle, transpile, or execute JS itself — it manages the packages your runtime of choice (Node, Bun, Deno) consumes.

---

## Design overview

```
┌──────────────┐      ┌──────────────┐      ┌──────────────┐
│   Resolver   │─────▶│   Fetcher    │─────▶│    Store     │
│  (pubgrub)   │      │  (reqwest)   │      │  (CAS, sha)  │
└──────────────┘      └──────────────┘      └──────────────┘
        │                                            │
        ▼                                            ▼
┌──────────────┐                            ┌──────────────┐
│  Lockfile    │                            │   Linker     │
│ (guroku.lock)│                            │ (hardlink)   │
└──────────────┘                            └──────────────┘
                                                    │
                                                    ▼
                                            ┌──────────────┐
                                            │ node_modules │
                                            └──────────────┘
```

- **Resolver** — version-constraint solver based on the [PubGrub](https://nex3.medium.com/pubgrub-2fb6470504f) algorithm.
- **Fetcher** — async HTTP client against the npm registry (or a configured mirror).
- **Store** — global content-addressable cache at `~/.guroku/store`, deduplicated by SHA-512.
- **Linker** — builds a strict, pnpm-style `node_modules` tree using hardlinks.
- **Lockfile** — deterministic `guroku.lock` for reproducible installs.

---

## Roadmap

### v0.1 — "It installs something" *(shipped 2026-05-06)*
- [x] CLI skeleton (`guroku install`, `guroku add`, `guroku remove`)
- [x] `package.json` parser
- [x] npm registry client (metadata + tarball download)
- [x] Tarball extraction (`.tgz` → store)
- [x] Naive flat `node_modules` writer
- [x] SHA-512 integrity verification

### v0.2 — "It resolves correctly" *(shipped 2026-05-06)*
- [x] Semver constraint parser (via `node-semver`)
- [x] Dependency resolver (BFS sticky-first; PubGrub-based variant deferred to v0.3 — see `docs/internals/algorithm-notes.md`)
- [x] `guroku.lock` writer/reader
- [x] Frozen-lockfile mode (`--frozen-lockfile`)
- [x] Dev dependency handling; peer / optional fields are read and round-tripped (auto-install lands in v0.4)

### v0.3 — "It's fast"
- [ ] Global content-addressable store at `~/.guroku/store`
- [ ] Hardlink-based linker
- [ ] Strict pnpm-style `node_modules/.guroku/<pkg>@<ver>` layout
- [ ] Parallel resolution + download pipeline
- [ ] HTTP response caching (ETag / If-None-Match)

### v0.4 — "It's usable"
- [ ] Lifecycle scripts (`preinstall`, `postinstall`, etc.)
- [ ] Workspaces / monorepo support
- [ ] `guroku run <script>`
- [ ] `guroku exec` / `guroku dlx`
- [ ] `.npmrc` compatibility (registry, auth tokens, scopes)

### v0.5 — "It plays nice"
- [ ] Private registry support (auth tokens, scoped registries)
- [ ] Git dependencies (`git+https://...`)
- [ ] Local / file dependencies (`file:../foo`)
- [ ] `package.json` overrides / resolutions
- [ ] `guroku audit` (advisory database lookup)

### v1.0 — "It's stable"
- [ ] Stable lockfile format
- [ ] Documented public Rust API for embedders
- [ ] Cross-platform CI (Linux, macOS, Windows)
- [ ] Pre-built binaries via GitHub Releases
- [ ] Benchmark suite vs. npm / pnpm / bun / yarn

### Future / maybe
- [ ] Plugin system
- [ ] Offline mode with pre-warmed store snapshots
- [ ] Deno / JSR registry support
- [ ] `guroku publish`

---

## Non-goals

- A JavaScript runtime or bundler. Use Node, Bun, Deno, esbuild, etc.
- Drop-in CLI parity with npm. Compatibility is a means, not the goal.
- Supporting every historical edge case in the npm ecosystem on day one.

---

## Building from source

> Requires Rust 1.75+.

```sh
git clone https://github.com/nktkt/guroku.git
cd guroku
cargo build --release
./target/release/guroku --help
```

---

## Contributing

This project is in its earliest stages, and the design is still in flux. If a roadmap item interests you, open an issue first to discuss the approach before sending a PR. Small, focused changes are easier to review than large rewrites.

---

## License

MIT
