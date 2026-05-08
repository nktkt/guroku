# Benchmark Methodology

This document describes how guroku v1.0 benchmarks are structured, how to run
them locally, and how we go about comparing guroku's install performance with
other JavaScript package managers (npm, pnpm, bun, yarn).

The short version: in v1.0 we ship **microbenches only**. They cover the hot
paths inside guroku itself (lockfile parsing, manifest parsing, spec
classification, semver matching) and run on every pull request. The
**macrobenches** -- end-to-end `guroku install` runs against a fixture project,
compared head-to-head with other installers -- are planned but **not** part of
v1.0. Until that harness exists, any "guroku is N times faster than X" number
should be treated as anecdotal.

---

## 1. Two kinds of benches

There are two distinct flavours of benchmark that we care about, and they
answer two different questions.

### 1.1 Microbenches

Microbenches live under `benches/` and are wired up via Cargo's
`[[bench]]` entries. They use [criterion](https://docs.rs/criterion) and run
**in-process**: a single binary loads the bench harness, calls into the
guroku crates, and measures the time spent inside specific functions.

Microbenches in v1.0:

- `lockfile_parse` -- deserialise a guroku lockfile of N packages.
- `manifest_parse` -- deserialise a representative `package.json`.
- `spec_classify` -- classify a single dependency spec string (registry,
  git, file, workspace, ...).
- `version_satisfies` -- test whether a single concrete version is contained
  in a single semver range.

Microbenches are **stable**: they take a fixed input, produce a fixed output,
and have no network or filesystem dependencies during the timed region. We run
them on every PR via `.github/workflows/bench-baseline.yml`, which records the
results and surfaces a regression if the new run drifts outside criterion's
noise band.

### 1.2 Macrobenches (planned, not in v1.0)

Macrobenches are end-to-end. They invoke `guroku install` (and the equivalent
command for npm, pnpm, bun, yarn) against a known fixture project, measure the
wall-clock time, and produce a comparison table.

The fixture project will live at `examples/benchmark-target/`. It is the
**single moving target** for cross-installer comparison: pinning the dependency
set means changes in the benchmark result reflect installer behaviour, not
upstream churn in the dep graph.

Macrobenches are **out-of-process**: the harness spawns the installer as a
subprocess, waits for it to exit, and looks at wall time. They are inherently
noisier than microbenches -- network, disk cache state, lifecycle scripts and
the OS scheduler all contribute -- which is why they are not gated on PR.

We do not ship macrobench results as part of v1.0 because the harness for
running all four competitors under identical conditions does not yet exist.
Adding it is tracked in section 9.

---

## 2. Running microbenches locally

The microbench suite is registered with Cargo, so the standard incantation is:

```sh
cargo bench
```

That runs every bench under `benches/`. To run a single bench by name:

```sh
cargo bench --bench lockfile_parse
```

To pass arguments through to criterion (e.g. to filter to a single benchmark
function within a bench binary), use `--`:

```sh
cargo bench --bench lockfile_parse -- 'parse/500'
```

The first run of `cargo bench` after a clean build will be slow because it
compiles guroku in release mode. Subsequent runs reuse the cached artefacts.

If you only want to make sure the benches still **compile** -- without paying
for the measurement loop -- you can do a no-op run:

```sh
cargo bench --no-run
```

This is what CI does on every PR to catch bench-only build breakages early.

---

## 3. What each microbench measures

Each microbench is deliberately narrow. None of them touch the network, and
none of them re-read input from disk inside the timed region: inputs are read
once into memory and passed to the function under test as a `&[u8]` or `&str`.

### 3.1 `lockfile_parse`

Measures the time to deserialise a guroku lockfile of N packages, for
N in {1, 50, 500}. The bench is parameterised on N so we can spot
super-linear behaviour creeping into the parser.

What it is sensitive to:

- JSON parse speed (`serde_json`).
- The shape of the `Lockfile` struct (large enums, `Vec<Vec<...>>`, etc).
- Allocator behaviour for the per-package metadata.

What it is **not** sensitive to:

- File I/O. We feed pre-loaded bytes; the open/read syscall is excluded.
- Disk layout, page cache state, or filesystem type.

If `lockfile_parse` regresses by more than the criterion noise band, the
likely culprit is either a structural change to `Lockfile` or a bump of
`serde_json`.

### 3.2 `manifest_parse`

Measures the time to parse a representative real-world `package.json` (we
use a snapshot of a moderately-sized project's manifest, checked in under
`benches/fixtures/`). This is faster than `lockfile_parse` -- a single
manifest is small -- but it runs on every install, so we care about its
constant factor.

### 3.3 `spec_classify`

Measures the time to take a single dependency spec string and classify it.
Examples of the inputs:

- `^1.2.3` -- registry range
- `1.2.3` -- registry exact
- `git+https://github.com/foo/bar.git#abcd` -- git
- `file:../local-pkg` -- file
- `workspace:*` -- workspace
- `npm:@scope/aliased@^2` -- aliased registry

The bench iterates through a representative mix and reports per-call time.
This is on the hot path during dependency-graph construction.

### 3.4 `version_satisfies`

Measures the time to test that one concrete version (e.g. `1.4.7`) is contained
in one semver range (e.g. `^1.2.0`). This is the inner loop of the resolver:
for a graph with R ranges and V candidate versions, we may call this O(R*V)
times. Even small constant-factor wins compound noticeably.

---

## 4. Reading criterion output

After `cargo bench` finishes, criterion writes a static HTML report to
`target/criterion/`. Open the index in a browser:

```sh
open target/criterion/report/index.html
```

(Or the equivalent `xdg-open` / `start` on Linux / Windows.)

Each metric reports:

- **Mean** -- the central estimate of the per-iteration time.
- **Standard deviation** -- spread across measurement samples.
- **Noise band** -- the threshold below which criterion considers a run
  to be statistical noise rather than a real change.

When you change code and re-run, criterion compares the new mean to the
previous run and labels the change as one of:

- *No change* -- inside the noise band.
- *Improved* / *Regressed* -- outside the noise band, with a confidence
  interval.

Treat "improved by 0.4%" with appropriate suspicion; treat "regressed by
12% with p < 0.01" as a real signal that warrants `git bisect`.

---

## 5. Comparing across runs

Criterion automatically diffs against the most recent run. That works for
casual back-and-forth ("did this commit make it faster?") but is fragile if
you switch branches or rebase, because the previous run might have been on
an unrelated tree.

For controlled comparisons, **save a named baseline**:

```sh
cargo bench -- --save-baseline before
```

This records the current run as the baseline named `before`. Now make your
change and re-run against that baseline:

```sh
# change code
cargo bench -- --baseline before
```

The report will compare the new run against `before` regardless of what other
runs happened in between. Use this whenever you are doing an A/B comparison
that involves any branch switching, dependency bumping, or toolchain change.

In CI we save a baseline per merged commit on `main`, which is what
`bench-baseline.yml` consumes when judging PRs.

---

## 6. Methodology for comparing with npm / pnpm / bun / yarn

This section describes the methodology macrobenches will use once the harness
lands. It is also the methodology you should follow if you are running ad-hoc
"is guroku faster than X?" measurements today, by hand.

The headline rule: **change exactly one variable at a time**. Anything else
produces results that are not reproducible and not interpretable.

### 6.1 Cold install

A cold install measures the path where nothing is cached: no `node_modules`,
no global package store, no metadata cache.

Procedure:

1. Remove any `node_modules` directory in the fixture project.
2. Remove every installer's global cache:
   - npm: `~/.npm`
   - pnpm: `~/.pnpm-store`
   - bun: `~/.bun/install/cache`
   - guroku: `~/.guroku/cas`
3. Run the installer under `time`:
   ```sh
   time npm install
   time pnpm install
   time bun install
   time yarn install
   time guroku install
   ```
4. Repeat the whole sequence (clean caches + install) **5 times**.
5. Report the **median**, not the mean. Cold installs have heavy tails;
   mean is misleading.

Cold-install numbers are dominated by network and tarball decompression,
not by installer logic. They are useful for "what does a fresh CI runner
see?" but not for evaluating the installer itself.

### 6.2 Warm install

A warm install measures the path where the global cache is hot but the
project's `node_modules` is not.

Procedure:

1. Run the installer once. Discard the timing -- this is just to warm the
   cache.
2. Remove `node_modules`:
   ```sh
   rm -rf node_modules
   ```
3. Run the installer **again**, under `time`. Record this number.
4. Repeat 5 times. Median.

Warm install is the more interesting comparison: it isolates the work the
installer does on the user's machine (graph construction, linking, lifecycle
scripts) from the variance of the network. It is also the path most users hit
day-to-day.

### 6.3 Same dep set

All comparisons must run against an **identical** `package.json` and lockfile.
Different installers produce different lockfiles, so you cannot share a single
lockfile across tools -- but the input `package.json` must be byte-identical,
and the dep set should be a published, well-known one (we are standardising on
`examples/benchmark-target/`).

Do not benchmark against your own product's `package.json` if it is changing
day-to-day. The dep set is a moving target then, and any speedup over time is
indistinguishable from "the deps got smaller".

### 6.4 Same network

Network latency to the registry is the single largest source of macrobench
variance. There are two ways to control it:

- **Local mirror**: run [Verdaccio](https://verdaccio.org) on `localhost`
  and point every installer at it. Latency becomes ~1 ms and roughly
  constant. This is the recommended setup.
- **Cached upstream**: pre-populate every installer's cache (cold-install
  once before the run starts) and only measure the warm path. Less
  reliable because installers may still talk to the registry for metadata
  freshness checks.

Do not run macrobenches over a coffee-shop wifi connection and post the
numbers. Half of the variance you see will be your wifi.

### 6.5 Same hardware

Run every comparison on a single fixed machine: a specific laptop, or a
specific cloud VM. **Never** compare numbers gathered on different hosts,
because:

- CI runners change underneath you (GitHub's hosted runners switched
  generation twice during 2024-2025).
- Laptop thermal throttling depends on ambient temperature.
- VM neighbours change cache and disk behaviour invisibly.

If you must use a cloud VM, pick a sized instance (e.g. `c7i.4xlarge`) and
keep it pinned for the duration of the measurement campaign.

### 6.6 Same Node

Lifecycle scripts (`postinstall`, `prepare`, ...) run under the project's
configured Node. The runtime version perturbs script start-up time, which
shows up directly in the wall-clock measurement. Pin Node version with
`nvm`:

```sh
nvm use 20
```

Use the same major.minor for **every** installer in the comparison. Mixing
Node 18 for npm and Node 20 for guroku is enough to invalidate a result.

---

## 7. What we DON'T claim to measure (yet)

Even when the macrobench harness lands, there are dimensions we are not
ready to publish numbers for. We mention them here so reviewers know what
*not* to take from a macrobench report.

### 7.1 Disk usage

How much space does `node_modules` occupy after install? Easy to script:

```sh
du -shx node_modules
```

But we are not yet automating this in the benchmark harness, and `du`
behaviour with hard links / clones (which CAS-based installers like guroku
and pnpm rely on) is filesystem-dependent. Comparing `du` numbers across
APFS, ext4 and ZFS is misleading.

### 7.2 Memory

Peak resident set size during install differs wildly between installers --
some stream, some buffer, some shell out to a managed runtime with its own
GC. Measuring it properly needs `valgrind --tool=massif` (Linux only) or
`/usr/bin/time -v` for max-RSS, and the numbers do not generalise across OSes.
Out of scope for v1.0 macrobenches.

### 7.3 CI throughput

"How fast is install on CI?" is a tempting question but a near-meaningless
one in isolation. Real-world CI numbers vary by 10x between, say, GitHub's
free `ubuntu-latest` runner and a Pro-tier machine, and another 5x between
"cold-cache CI" and "with-cache CI". We are not going to publish CI numbers
because they cannot be interpreted without knowing the runner class.

---

## 8. Reproducibility checklist for benchmark results

If you are sharing a benchmark number publicly -- in a blog post, a release
note, an issue comment -- include all of the following so a reader can
re-run and check.

- **Source**:
  - Which guroku tag or commit was tested.
  - Which version of every competing installer (`npm --version`,
    `pnpm --version`, `bun --version`, `yarn --version`).
- **Hardware**:
  - Laptop model (e.g. "MacBook Pro 14-inch M3 Pro, 36 GB RAM") or
    VM type (e.g. "AWS c7i.4xlarge").
  - OS and version (`sw_vers` on macOS, `lsb_release -a` on Linux,
    `winver` on Windows).
- **Network**:
  - Local Verdaccio? Direct to npmjs.org? Corporate mirror?
  - Approximate round-trip latency to the registry endpoint.
- **Sample size**:
  - **At least 5 runs** per installer per scenario.
  - **Report median**, not mean. Optionally include the full distribution
    (min, p25, p50, p75, max).
- **Caveats**:
  - Any errors observed during the runs (e.g. one installer failed and
    had to be retried -- that retry is not in the median).
  - Any non-default flags passed to any installer.
  - Any `.npmrc` / `.guroku.toml` overrides in the environment.

A benchmark number that doesn't include these is not reproducible, and we
will not cite it.

---

## 9. Future work

Tracked roughly in priority order.

- **Macrobench harness**: a single Rust binary that drives all four
  installers (and guroku) against the fixture, in a controlled order, with
  cache reset between cold runs. Emits a structured JSON result so the
  CI step can post a comparison table.
- **Release-tag GitHub Action**: on every release tag, run the macrobench
  harness on a fixed runner class and post the comparison numbers as part
  of the release notes. Because the runner is fixed, run-to-run noise is
  bounded.
- **Synthetic-load corpus**: a corpus of real-world `package.json` files
  (anonymised), against which we can run the macrobench harness to show
  performance across a range of project shapes -- not just one fixture.
- **Disk usage automation** (section 7.1) once we have a story for
  comparing across filesystems.
- **Memory measurement** (section 7.2) once we have a cross-platform
  approach. Probably not until v1.2.

None of these are gating for v1.0.

---

## 10. Why we don't publish numbers in the README

Until the macrobench harness in section 9 exists, **any single number we
publish is anecdotal**. It is one person, on one laptop, on one network,
running one fixture, on one day. That is exactly the scenario we tell other
people to avoid in section 8.

The microbenches are useful internally -- they catch regressions in the
parser, the resolver inner loop, the spec classifier -- but they measure
**individual pieces of the install pipeline**, not the user-visible install
time. A 30% speedup in `version_satisfies` does not turn into a 30% speedup
in `guroku install`, because `version_satisfies` is a small share of total
install time. Conversely, a 5% slowdown in `lockfile_parse` is invisible at
the install-command level but very visible at the `cargo bench` level.

The two benchmark layers are answering different questions, and we want both
running before we make claims about either.

When the macrobench harness lands and has produced stable numbers across at
least three release tags, we will publish a comparison table -- with all of
section 8 attached -- in the project README. Until then, the README stays
silent on performance, and this document stays the canonical reference for
how we measure ourselves.
