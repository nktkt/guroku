# guroku

> A fast, Rust-powered package manager for the JavaScript ecosystem.

`guroku` is an experimental package manager inspired by [npm](https://www.npmjs.com/), [pnpm](https://pnpm.io/), and [bun](https://bun.sh/), built from scratch in Rust. The goal is to combine pnpm's content-addressable storage model with bun's installation speed, while staying small, hackable, and easy to read.

> **Status:** v1.2 — "It backtracks properly." First feature minor that ships a real PubGrub-based resolver as the default. Cascading conflicts that defeated v1.1's single-step backtracking now resolve cleanly when a solution exists, and produce structured derivation reports when one doesn't. The lockfile schema, `guroku::prelude` surface, and CLI are unchanged from v1.0. Set `GUROKU_RESOLVER=bfs` to force the v1.1 BFS path. See [`docs/v1.2-release-notes.md`](docs/v1.2-release-notes.md) and [`docs/pubgrub-resolver.md`](docs/pubgrub-resolver.md).

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

### v0.3 — "It's fast" *(shipped 2026-05-06)*
- [x] Global content-addressable store at `~/.guroku/cas/<sha[0:2]>/<sha[2:]>`
- [x] Hardlink-based linker (with copy fallback for cross-fs cases)
- [x] Strict pnpm-style `node_modules/.guroku/<pkg>@<ver>` layout
- [x] Parallel resolution prefetch + parallel CAS download pipeline
- [x] HTTP response caching (ETag / If-None-Match)

### v0.4 — "It's usable" *(shipped 2026-05-06)*
- [x] Lifecycle scripts (`preinstall`, `install`, `postinstall`, `prepare`)
- [x] Workspaces — discovery (`guroku workspaces`); inter-dep linking deferred to v0.5
- [x] `guroku run <script>` (with `-- args` forwarding)
- [x] `guroku exec <cmd>`; `guroku dlx` deferred to v0.4.x
- [x] `.npmrc` reading (registry + scoped registry); `_authToken` parsed, sent on requests in v0.5

### v0.5 — "It plays nice" *(shipped 2026-05-08)*
- [x] Private registry support — `_authToken` bearer auth + `<scope>:registry=` routing
- [x] Git dependencies (`git+https://`, `git+ssh://`, `github:user/repo[#ref]`)
- [x] Local / file dependencies (`file:./path`)
- [x] `package.json#overrides` / `resolutions` — simple flat-name → exact-version form (path-keyed and glob forms parse but aren't matched yet)
- [x] `guroku audit` — npm advisories bulk endpoint

### v1.0 — "It's stable" *(shipped 2026-05-08)*
- [x] Stable lockfile format (`lockfileVersion: 1` covered by SemVer; forward-compat tests in place)
- [x] Documented public Rust API for embedders (`guroku::prelude` + comprehensive rustdoc)
- [x] Cross-platform CI (Linux + macOS in `ci.yml`; Win/Mac/Linux matrix in `cross-platform-test.yml`)
- [x] Pre-built binaries via GitHub Releases (Linux x86_64/aarch64, macOS x86_64/aarch64, Windows x86_64/aarch64)
- [x] Benchmark suite scaffolding (criterion microbenches under `benches/`); macrobench harness vs. npm/pnpm/bun/yarn deferred to v1.x — see `docs/benchmark-methodology.md`

### v1.1 — "It resolves better" *(shipped 2026-05-08)*
- [x] npm-style aliases (`"react-old": "npm:react@^16"`) — `DepSpec::Alias`, classifier splits on the last `@` so scoped real names work
- [x] Path-keyed overrides (`"a > b > c": "1.0.0"`) with whitespace tolerance
- [x] Yarn-style glob resolutions (`"**/foo": "1.0.0"`)
- [x] Single-step backtracking on diamond conflicts; `ResolutionConflict.requested_by` now formats the dep-graph path as `"a > b > c"`
- [x] `resolver::resolve_with_manifest_overrides` — the manifest-aware entry point that wires the new override forms through
- [x] Strictly additive on the v1.0 stability surface — full PubGrub-the-crate integration deferred to v1.2

### v1.2 — "It backtracks properly" *(shipped 2026-05-08)*
- [x] `pubgrub = "0.2"` integration via the new `guroku::pubgrub_resolver` module
- [x] `NpmVersion` newtype implementing `pubgrub::version::Version`
- [x] Two-phase async-prefetch + sync-solve bridge
- [x] Candidate-set range translation (`docs/internals/range-conversion.md`)
- [x] DefaultStringReporter conflict reports surfaced via `ResolutionConflict.requested_by`
- [x] `GUROKU_RESOLVER=bfs` opt-out preserves the v1.1 path
- [x] file:/git: roots transparently fall back to v1.1 BFS

### v1.3 — "Workspaces, properly"
- [ ] Cross-workspace symlinks (skip the CAS for sibling workspace packages)
- [ ] Topological `guroku run -r <script>` with stream-prefixed output
- [ ] Workspace-scoped lockfile (single root lockfile covering every workspace)
- [ ] `workspace:^`, `workspace:*`, `workspace:~` protocols

### v1.4 — "It runs offline"
- [ ] `guroku store export <out.tar>` / `import <in.tar>` for portable CAS bundles
- [ ] `--offline` flag for `install` (no network calls; errors if the CAS is incomplete)
- [ ] `guroku store gc` over a configurable scan list

### v1.5 — "It publishes"
- [ ] `guroku publish` (npm-compatible tarball + integrity + provenance + dist-tag)
- [ ] 2FA / OTP flows
- [ ] Sigstore-backed `--provenance` matching npm's format
- [ ] `guroku version` (mirror of `npm version`)

### v1.6 — "Plugins"
- [ ] WASM-component plugin host with explicit capabilities
- [ ] Per-project plugin enablement via `.gurokurc.json`
- [ ] CLI extension points (`guroku <plugin-name> <args>`)
- [ ] Lifecycle hook extension points (`pre-resolve`, `post-resolve`, `pre-link`, `post-link`, `pre-script`, `post-script`)

### Future / maybe
- [ ] Deno / JSR registry support

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
