# Parallelism in guroku

This document describes where guroku runs work concurrently, where it does
not, and why. It tracks the state of the codebase as of v0.3.

The short version: guroku is concurrent in the two places that matter most
for wall-clock install time (root metadata prefetch and CAS fills) and
deliberately serial everywhere else. We have not chased parallelism for its
own sake.

## 1. Tokio multi-thread runtime

The binary entrypoint is annotated with the default `#[tokio::main]`:

```rust
#[tokio::main]
async fn main() -> Result<()> {
    cli::run().await
}
```

We do not pass `flavor = "current_thread"` and we do not configure a custom
runtime. That means tokio gives us its multi-thread scheduler with a worker
count equal to the number of logical CPUs. Anywhere we `.await` something
network- or filesystem-bound, the runtime is free to park that task and run
another one on the same OS thread, and any blocking work we hand off via
`spawn_blocking` lands on the blocking pool.

Practical consequence: every `async fn` in guroku is allowed to interleave
with every other `async fn` for free. We do not need to hand-thread an
executor or worry about which thread a future is pinned to.

## 2. Resolver: parallel root prefetch (v0.3)

`resolver::resolve` begins with a prefetch pass over the direct
dependencies declared in `package.json`:

```rust
pub async fn resolve(
    client: &RegistryClient,
    root_deps: &BTreeMap<String, VersionReq>,
) -> Result<Resolution> {
    let root_names: Vec<&str> = root_deps.keys().map(String::as_str).collect();
    prefetch(client, &root_names).await;

    // ... BFS walk follows
}
```

`prefetch` builds a `FuturesUnordered<_>` of metadata fetches and drains it:

```rust
async fn prefetch(client: &RegistryClient, names: &[&str]) {
    let mut futs = FuturesUnordered::new();
    for name in names {
        futs.push(client.fetch_packument(name));
    }
    while let Some(_res) = futs.next().await {
        // results are cached inside RegistryClient; we discard here
    }
}
```

Every root package's packument is requested concurrently. The registry
client's internal cache absorbs the responses, so when the BFS later asks
for those same packuments it gets a hit instead of issuing a second
request.

This is the single biggest win in the resolver. A typical `package.json`
has 20 to 60 direct dependencies; doing them serially round-trips you 20
to 60 times before the BFS even starts.

## 3. Why we don't go fully parallel in the BFS

After the prefetch, the BFS itself is serial. It walks a queue of
`(name, range)` pairs, picks a concrete version, reads that version's
declared dependencies out of the cached packument, and pushes the new
`(name, range)` pairs onto the queue.

You could imagine fanning out at every BFS frontier instead. We don't,
because the input to each fan-out depends on the output of the previous
level: every transitive depends on knowing what its parent resolved to,
so we can fan out the right `(name, range)` queries. Starting them
earlier means starting them with stale info, and we then either redo
work or commit to a wrong version.

The prefetch dodges this because we already know the root names from
`package.json` directly; we are not waiting on a resolver decision to
form the request. By the time the BFS walks transitives, most of them
have been requested anyway as a side effect of fetching the roots'
packuments (which contain their full dependency lists). So the serial
BFS is mostly cache hits in practice. We accept the residual wall-clock
cost.

## 4. Install pipeline: parallel CAS fills

`commands::install::install_from_resolution` turns the resolved set into a
list of "fetch this tarball into the CAS" jobs and runs them with bounded
concurrency:

```rust
let items: Vec<CasJob> = resolution
    .packages
    .iter()
    .map(CasJob::from)
    .collect();

stream::iter(items)
    .map(|job| fetch_into_cas(client.clone(), store.clone(), job))
    .buffer_unordered(8)
    .try_collect::<Vec<_>>()
    .await?;
```

`buffer_unordered(8)` is the parallelism knob. Up to eight `fetch_into_cas`
futures are in flight at any moment, and as soon as one completes another
takes its slot.

Each `fetch_into_cas` call:

1. Downloads the tarball over HTTP.
2. Verifies the integrity hash.
3. Extracts into a tmp directory.
4. Renames into the final CAS path.

All four steps happen inside the future, so a slow extract on one package
does not block a fast download on another.

## 5. Why 8

The number is empirical. The npm registry's per-IP rate limits start
kicking in around 16 concurrent connections, beyond which you start
seeing 429s and degraded throughput. 8 leaves headroom for retries
without ever touching the limit, and it keeps the typical residential
upstream pipe saturated for tarball-sized payloads.

Fewer than 8 leaves bandwidth unused on most installs. More than 8
saturates the registry rate-limit and starts costing you wall-clock time
to back-offs.

If you are running guroku against a private registry with different
limits, this is a reasonable thing to make configurable later. For now
it is a constant.

## 6. Where things stay serial

Some parts of the pipeline are explicitly single-threaded.

- **Linker (`populate_node_modules`)** writes sequentially. It walks
  the resolution and issues `hardlink` calls one at a time. Hardlinks
  are cheap kernel operations, the bookkeeping for "did I already link
  this?" is simpler in a serial loop, and we have not seen this be a
  bottleneck in practice.
- **Lockfile writes** happen once per install at the end. There is
  nothing to parallelize: it is a single serialize-and-write to
  `guroku.lock`.
- **`manifest.read_from` / `manifest.write_to`** operate on `package.json`,
  a small file, with single-threaded I/O. Async would not buy us
  anything.

These are deliberate choices, not oversights. Adding concurrency here
would buy us micro-optimizations at the cost of harder-to-reason-about
code.

## 7. CAS race handling

Because multiple guroku processes can run on the same machine (think
`cargo test` with several integration tests in flight, or a developer
running `guroku install` in two checkouts at once), two processes can
race on the same content hash.

The store handles this via tmp-then-rename:

```rust
let tmp = store.tmp_dir_for(&hash);
extract_tarball_into(&tmp, &tarball)?;
match std::fs::rename(&tmp, &final_path) {
    Ok(()) => Ok(()),
    Err(e) if final_path.exists() => {
        // someone else won the race; clean up our tmp and continue
        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }
    Err(e) => Err(e.into()),
}
```

Both processes extract into siblings, both attempt rename, the loser
cleans up its tmp directory. The winning rename is atomic on every
filesystem we support, so readers never see a half-populated CAS entry.

This is what makes `cargo test` safe even when multiple test runs hit
the same fixture: they cooperate without locks.

## 8. Future parallelism

Things we have looked at and not yet done:

- **Streaming tarball download into the extractor.** Right now
  `fetch_into_cas` buffers the full tarball in memory and then hands it
  to the tar reader. A streaming pipe would let extraction start before
  download finishes, halving the latency on large packages.
- **Pipelined "download N+1 while extracting N".** Inside a single
  `fetch_into_cas` future, the download and extract phases are
  sequential. They could overlap.
- **Concurrent walks of `populate_node_modules`.** One task per package
  directory would parallelize the linker. We would need to think about
  ordering for nested `node_modules`, but it is tractable.

None of these are blocked on architecture; they are just not yet worth
the complexity given current install times.

## 9. Diagnostics

To see the parallelism in action, run with debug logging:

```sh
GUROKU_LOG=debug guroku install
```

This prints every metadata fetch and every CAS write with timestamps.
The interleaving is visible in the timeline: you will see eight CAS
writes in flight at once during the install phase, and you will see
the root packument fetches all dispatched in a single burst at the
start of resolution. If the timeline does not look like that, something
is wrong with the runtime configuration.
