# Getting Started with guroku

This guide takes you from "I just heard about guroku" to "I have a `node_modules/`
populated by guroku" in about ten minutes. guroku is an npm-style package manager
written in Rust; it reads `package.json`, talks to the npm registry, and lays
packages out into a familiar `node_modules/` tree.

## 1. Status: pre-alpha

guroku is **pre-alpha software**. Do not use it for production projects, do not
point it at your day job's monorepo, and do not rely on it as your only package
manager.

In particular, several pieces that you would expect from a mature package
manager are **not yet implemented**:

- **Resolver** — version resolution is intentionally minimal. Anything that
  cannot be parsed as an exact version currently falls back to the latest
  release on the registry. There is no SAT solver, no peer-dependency logic,
  and no deduplication strategy beyond the content-addressed store.
- **Lockfile** — there is no `guroku.lock` (or equivalent) yet. Two installs of
  the same `package.json` may pick up different transitive versions if upstream
  publishes between runs.
- **Hardlinks / reflinks** — packages are *copied* from the global store into
  `node_modules/`. The plan is to hardlink (and reflink on supported
  filesystems) the way pnpm does, but that work is still on the roadmap.

See the **Roadmap** section of the [README](../README.md) for the current
priority order, and [docs/ARCHITECTURE.md](./ARCHITECTURE.md) for what the
finished pipeline is meant to look like.

## 2. Install

There are no published binaries yet. Build from source:

```sh
git clone https://github.com/nktkt/guroku.git
cd guroku
cargo build --release
sudo install -m 0755 target/release/guroku /usr/local/bin/   # optional
```

Requirements:

- **Rust 1.75 or newer** (stable toolchain). `rustup update stable` is the
  easiest way to make sure you are current.
- A working network connection to `https://registry.npmjs.org`.

If you skip the `sudo install` step, run guroku as
`./target/release/guroku ...` from the checkout, or add the `target/release`
directory to your `PATH`.

Verify the build:

```sh
guroku --help
```

## 3. A 90-second tour

Spin up a throwaway project and add a real dependency:

```sh
mkdir hello-guroku && cd hello-guroku
echo '{"name":"hello-guroku","version":"0.0.1"}' > package.json
guroku add lodash
ls node_modules/
```

You should see `lodash/` (and any of its transitive dependencies) sitting under
`node_modules/`. Your `package.json` will have been updated to record `lodash`
in the `dependencies` map.

That is it. Same input, same shape of output as npm — just a different binary
on the front of it.

## 4. What just happened?

The `guroku add` command ran the same install pipeline that `guroku install`
uses, with one extra step at the start (writing the new dependency into
`package.json`). The pipeline is:

1. **Read manifest** — parse `package.json` to discover the dependency set.
2. **Fetch metadata** — request package documents from the npm registry.
3. **Resolve** — pick a concrete version for each requested range. Today this
   is "exact version, or fall back to latest"; eventually this will be a real
   resolver.
4. **Fetch tarball** — download each resolved package's `.tgz` from the
   registry CDN.
5. **Verify SHA-512** — check the tarball's hash against the integrity field
   from the registry metadata. Mismatches abort the install.
6. **Extract to the global store** — unpack into `~/.guroku/store`,
   content-addressed by integrity hash, so each `name@version` lives on disk
   exactly once per machine.
7. **Materialize `node_modules/`** — copy from the store into your project's
   `node_modules/` tree.

For the longer version, including data structures and module boundaries, read
[docs/ARCHITECTURE.md](./ARCHITECTURE.md).

## 5. Common commands

The everyday surface is small:

| Command            | What it does                                                       |
| ------------------ | ------------------------------------------------------------------ |
| `guroku install`   | Install everything declared in `package.json`.                     |
| `guroku add <pkg>` | Add `<pkg>` to `dependencies` and install it.                      |
| `guroku remove <pkg>` | Remove `<pkg>` from `dependencies` and from `node_modules/`.    |
| `guroku --help`    | Top-level help. Each subcommand also accepts `--help`.             |

Full flag-by-flag documentation lives in
[docs/cli-reference.md](./cli-reference.md).

## 6. Logging

guroku uses a `GUROKU_LOG` environment variable for log filtering, in the same
spirit as `RUST_LOG`. To see the install pipeline narrate itself:

```sh
GUROKU_LOG=debug guroku install
```

Useful levels are `error`, `warn`, `info`, `debug`, and `trace`. `debug` is
usually the right setting when something is going wrong; `trace` is loud enough
to be hard to read but is invaluable when filing a bug.

## 7. Where things live

After a successful install, three locations matter:

- `./node_modules/` — the per-project tree your Node.js runtime resolves
  against. Safe to delete; `guroku install` rebuilds it.
- `./package.json` — the manifest. guroku rewrites the `dependencies` map when
  you run `add` or `remove`, and otherwise leaves it alone.
- `~/.guroku/store/` — the global content-addressed store. Tarballs are
  extracted here once and reused across every project on the machine. Safe to
  delete (it will be repopulated on the next install), but doing so will force
  re-downloads.

## 8. Troubleshooting

### Integrity check failed

If guroku reports a SHA-512 mismatch on a tarball, it aborts the install
rather than write a corrupt package to disk. This is the desired behavior.

What to try, in order:

1. Re-run the install. A truncated download is the most common cause and a
   second attempt usually succeeds.
2. Clear the offending entry (or the whole store) and retry:
   `rm -rf ~/.guroku/store`.
3. If it still fails on the same package and version, you may have hit either
   a network middlebox rewriting bytes or — much less likely — a genuinely
   bad publish. File an issue with the package name, version, and the hash
   guroku reported.

### Registry is unreachable

If `https://registry.npmjs.org` is blocked, slow, or returning errors:

1. Confirm with `curl -I https://registry.npmjs.org/`. If that fails, the
   problem is outside guroku.
2. Check whether a corporate proxy or VPN is in the way; guroku honors the
   standard `HTTPS_PROXY` / `HTTP_PROXY` environment variables.
3. Re-run with `GUROKU_LOG=debug` to see the exact URL and HTTP status that
   failed.

guroku does not currently support alternate registries via configuration; that
is on the roadmap.

### A package's version spec falls back to latest

If you ask for `"lodash": "^4.17.0"` and guroku installs `4.17.21` (or whatever
is current at install time), that is **expected for now**. Until the resolver
lands, anything that is not an exact version (`4.17.21`) is treated as "give me
the latest". With `GUROKU_LOG=debug` you will see a log line announcing the
fallback for each affected dependency.

If reproducibility matters for your experiment, pin every dependency to an
exact version in `package.json` until the resolver and lockfile are
implemented.

## 9. Next steps

- Read [docs/ARCHITECTURE.md](./ARCHITECTURE.md) to understand how the install
  pipeline is wired together internally.
- Read [CONTRIBUTING.md](../CONTRIBUTING.md) if you want to send a patch — it
  covers the development setup, the test suite, and the commit conventions.
- File an issue at <https://github.com/nktkt/guroku/issues> for anything that
  is broken, unclear, or missing. Bug reports with a minimal `package.json`
  and a `GUROKU_LOG=debug` transcript are especially welcome.

Welcome to guroku.
