# benchmark-target

A reference fixture for comparing guroku v1.0's install performance against
other JavaScript package managers.

## What this example shows

This package is a deliberately simple, reproducible fixture used to compare
guroku v1.0's wall-clock install time against npm, pnpm, bun, and yarn. It is
not a real application; it is a `package.json` plus this README. The intent is
to give every installer the same input so that timing differences reflect the
installer itself, not differences in the dependency graph.

If you are evaluating guroku for adoption, or you want to reproduce the
numbers we publish on each release, this is the fixture to point your stopwatch
at.

## The dep set

The fixture pins 11 runtime dependencies and 1 dev dependency. The set was
chosen with three properties in mind:

- **Small.** Small enough that a full cold install completes in seconds, not
  minutes, on a modern laptop. This keeps the loop tight when running 5+
  iterations for a median.
- **Real-world-ish.** The packages are ones you'd plausibly find in a small
  Node service or CLI: an HTTP client, a logger, a validator, a date library,
  a few utilities. Synthetic graphs (`left-pad` x 200) tend to flatter
  installers that optimize for trivial graphs and tell you nothing useful.
- **Reproducible.** Every version is pinned exactly (no `^`, no `~`, no
  ranges). This means every installer resolves the same versions, so the only
  variable is how the installer fetches, extracts, and links them.

See `package.json` for the exact list. If you change it, the numbers are no
longer comparable to the published baselines.

## How to run a comparison

```sh
cd examples/benchmark-target

# Cold caches
rm -rf node_modules guroku.lock package-lock.json pnpm-lock.yaml yarn.lock
rm -rf ~/.guroku ~/.npm ~/.local/share/pnpm ~/.bun/install/cache

# Time each installer (pick a recent stable of each)
time guroku install
rm -rf node_modules
time npm install
rm -rf node_modules
time pnpm install
rm -rf node_modules
time bun install
rm -rf node_modules
time yarn install
```

Run the block above as a single shell script if you want a clean record of
all five installers in one log.

## How to read the timings

The `time` builtin prints three numbers:

- `real` — wall-clock time from start to finish. This is what the user
  actually waits for, and it is the only number that matters for installer
  comparison.
- `user` — CPU time spent in user space across all threads. A heavily
  parallel installer can show `user` > `real`.
- `sys` — CPU time spent in kernel space (syscalls, I/O setup, etc.).

For installer comparison, report `real`. Don't average `user` and `sys`; they
measure CPU work, not perceived speed.

## Methodology recommendations

- Run 5 times; take the median. The first run is often slower (cold OS page
  cache, DNS), and a single tail-latency network hiccup can poison a mean.
- Same hardware, same network, same wall power state. Laptops on battery
  throttle; results will not be comparable to AC runs.
- Same Node version (`nvm use 20` or pin in `.nvmrc`). Some installers'
  lifecycle scripts shell out to `node` and perturb timings depending on the
  Node version installed.
- Use a local Verdaccio mirror to hold registry latency constant. This is
  best practice for serious comparisons: it removes public-registry jitter
  and makes runs from different days comparable. Point every installer at
  the same Verdaccio URL via `.npmrc` / `bunfig.toml` / `.yarnrc.yml`.
- Disable any background sync or indexer (Spotlight, Dropbox, antivirus) on
  the directory under test. These can add hundreds of ms per run.

## Warm vs cold

The block above measures the cold case: no global cache, no lockfile, no
`node_modules`. To measure the warm case — what users actually experience on
their second install — keep global caches populated and only clear the local
tree:

```sh
rm -rf node_modules
time guroku install
rm -rf node_modules
time npm install
# ...etc
```

guroku v1.0's content-addressable store shines in the warm case: most files
are already on disk and the install reduces to hardlink/clone operations.
Cold numbers tell you about the network and tarball pipeline; warm numbers
tell you about the linker. Publish both.

## What this fixture does NOT measure

This is a wall-clock install benchmark and nothing more. It does not
measure:

- **Disk usage.** If you want that number, run `du -shx node_modules` after
  each installer, or wrap the runs in a small script.
- **Memory.** Use `valgrind massif` on Linux, Instruments on macOS, or the
  platform equivalent. Peak RSS during install is a separate metric.
- **Time to first dev-loop iteration.** The number a developer actually
  feels is `guroku install && npm test` (or your equivalent). That ratio
  depends on the test suite, not the installer alone.
- **Correctness.** Two installers that produce different `node_modules`
  trees can both be "right" by their own rules. Timing them against each
  other says nothing about which graph is correct.

## Caveats

- Different installers handle lifecycle scripts differently. npm and yarn
  run them by default; pnpm and bun have varied defaults across versions;
  guroku v1.0 runs them by default. For comparable numbers, run every
  installer with `--ignore-scripts` (or its equivalent) and note this in
  the published result.
- Some packages have `optionalDependencies` that fail on certain platforms
  (e.g. `fsevents` on Linux). The fixture above intentionally avoids those
  so that no installer wastes time on a fetch-then-fail path that another
  installer skipped.
- Installers with workspaces enabled may behave differently from
  single-package mode. This fixture has no `workspaces` field, so every
  installer takes the single-package code path.
- Lockfile presence changes timings. The cold block above deletes all
  lockfiles; if you want lockfile-resolved timings, generate each
  installer's lockfile once and keep it for subsequent runs.

## Related docs

For the deeper discussion — why we chose these specific packages, how we
configure Verdaccio, what statistical tests we run on the resulting samples,
and how we publish numbers per release — see `docs/benchmark-methodology.md`
in the guroku repo.

## Future work

We are building a Rust harness that drives all four external installers
(npm, pnpm, bun, yarn) plus guroku from a single process, runs N iterations,
discards warmup, computes median + p95 + IQR, and posts the comparison
numbers as part of every guroku release. The harness will live under
`tools/benchmark-harness/` and consume this fixture (and a few larger ones)
as inputs. When that ships, the manual procedure documented above becomes a
fallback for ad-hoc local checks rather than the primary measurement path.
