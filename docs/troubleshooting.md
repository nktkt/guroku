# Troubleshooting

This guide covers common problems people hit when using guroku v0.3, what
they usually mean, and how to get unstuck. If your symptom isn't listed
here, see `docs/error-codes.md` for the full error reference, or open an
issue with the failing command and the full error output.

The sections below are roughly ordered from most-frequently-reported to
least-frequently-reported.

---

## `Error creating symbolic link` on Windows

### Symptom

On Windows, install fails partway through with a message like:

```
error: Error creating symbolic link from
'C:\...\.guroku\cas\sha512-...\node_modules\lodash' to
'C:\project\node_modules\lodash': A required privilege is not held by the client. (os error 1314)
```

### Cause

guroku's strict layout (the default in v0.3) is built on symlinks. Most
modern operating systems let unprivileged users create symlinks, but
Windows does not by default — creating a symbolic link is a privileged
operation unless Developer Mode is on, or the process is running
elevated.

### Fix

Enable Developer Mode:

1. Open **Settings**.
2. Go to **Update & Security** -> **For developers**.
3. Toggle **Developer Mode** on.

You do not need to restart, but you do need to start a new shell so it
picks up the new privilege. Then re-run `guroku install`.

If you cannot enable Developer Mode (locked-down corporate machine),
your only option in v0.3 is to run the install in an elevated shell.
The flat-layout escape hatch (`--use-flat-layout`) is planned for v0.4
and is not available yet.

---

## `integrity check failed for X@Y`

### Symptom

```
error: integrity check failed for left-pad@1.3.0
  expected sha512-abcd...
  got      sha512-wxyz...
```

### Cause

guroku verifies every tarball it downloads against the SHA-512 recorded
in the registry metadata (and, for subsequent installs, against the
lockfile). If the bytes on disk hash to something different, the check
fails and the install aborts before anything is linked into your
project.

In practice the cause is almost always one of:

- **A flaky CDN.** The tarball was truncated or served from a stale
  edge. This is by far the most common case, and it goes away on a
  retry.
- **A corrupt registry record.** Rare, but it does happen — usually
  right after a registry incident.
- **Local cache corruption.** A previous install was killed while a
  tarball was being written.

### Fix

First, retry:

```sh
guroku install
```

If it fails again on the same package, clear the relevant cache entry
and retry:

```sh
rm -rf ~/.guroku/cas
guroku install
```

If it still fails on the same package and same version, the registry
record is likely bad. File an issue with the package name, the version,
and the full error output. Pinning to an adjacent version
(`X@Y-1`) is a reasonable workaround in the meantime.

---

## `no matching version for X@^Y`

### Symptom

```
error: no matching version for chalk@^99.0.0
```

### Cause

The version range in your `package.json` (or in some transitive dep's
`package.json`) does not match any version that the registry has
actually published. Common reasons:

- A typo in the version (`^99.0.0` instead of `^9.0.0`).
- A package was unpublished or deprecated.
- A pre-release that was never promoted to a stable version.

guroku v0.3's resolver does not try alternative ranges; if nothing in
the published list satisfies the range, it gives up.

### Fix

Check what's actually published:

```sh
npm view chalk versions
```

(Yes, `npm view` — guroku v0.3 doesn't ship a `view` subcommand yet.)

Then either widen the range to include something that exists, or pin
to a real version:

```json
{
  "dependencies": {
    "chalk": "^5.3.0"
  }
}
```

---

## `lockfile is out of date with package.json`

### Symptom

In CI, with `--frozen-lockfile`:

```
error: lockfile is out of date with package.json
  manifest declares: react@^18.2.0
  lockfile resolves: (none)
```

### Cause

You added (or removed, or bumped) something in `package.json` and
forgot to commit the updated `guroku.lock`. `--frozen-lockfile` makes
this a hard error on purpose — it's the flag you want in CI, because
it guarantees reproducibility.

### Fix

Locally, run install once **without** `--frozen-lockfile`, commit the
updated lockfile, and push:

```sh
guroku install
git add guroku.lock
git commit -m "update guroku.lock"
git push
```

Then re-run CI. The frozen install should now succeed.

---

## `lockfile version mismatch: file is vN, this guroku understands v1`

### Symptom

```
error: lockfile version mismatch: file is v2, this guroku understands v1
```

### Cause

Someone on your team (or your CI image) is using a newer guroku than
you are. The newer guroku produced a lockfile in a format your
version doesn't know how to read. v0.3 understands lockfile v1 only.

guroku does not auto-downgrade lockfiles — that would silently lose
information.

### Fix

Upgrade guroku. If you installed via `cargo install`:

```sh
cargo install --force guroku
```

If you're pinning a guroku version in CI, bump the pin to match what
your teammates are running.

---

## `version conflict for X`

### Symptom

```
error: version conflict for lodash
  required by foo@1.2.3:        ^4.17.0
  required by bar@2.0.0:        ^3.0.0
  no single version satisfies both
```

### Cause

Two transitive paths in your dependency graph want incompatible ranges
of the same package. npm and pnpm handle this by installing both
copies — one nested inside each parent. guroku v0.3's resolver does
not backtrack and does not nest by default; it picks one version per
name and errors out if it can't.

This is a deliberate v0.3 limitation, not a bug. The plan for v0.4 is
to add a `--force` flag that picks the highest range and installs
nested copies for parents that disagree.

### Fix

You have a few workarounds today:

- **Pin one of the parents** to a version whose dep range agrees with
  the other side. If `foo@1.2.4` requires `lodash@^3` (matching
  `bar`), pin `foo@1.2.4` in your `package.json`.
- **Drop one of the parents** if you can.
- **Wait for v0.4** and use `--force` (not yet available).

If you regularly hit this, open an issue with the conflict — the
resolver team is collecting real-world cases to drive the v0.4 design.

---

## `os error 13: Permission denied` during install

### Symptom

```
error: failed to write /Users/you/.guroku/cas/sha512-.../package.tgz
  caused by: Permission denied (os error 13)
```

### Cause

guroku can't write into `~/.guroku`. Almost always this is because
something earlier (often a `sudo guroku install` you ran once and
forgot about) created files in `~/.guroku` owned by root, and now your
non-root user can't write there.

### Fix

Check ownership:

```sh
ls -la ~/.guroku
```

If anything is owned by `root` (or by a different user), fix it:

```sh
sudo chown -R $USER ~/.guroku
```

Then retry the install. As a rule: do not run guroku under `sudo`.
guroku writes to `~/.guroku` and to `./node_modules`, both of which
should be owned by the user running the install.

---

## `Operation not permitted` on hardlink

### Symptom

```
error: failed to hardlink
  /Users/you/.guroku/cas/sha512-.../package.json
  -> /Volumes/Work/project/node_modules/.guroku/.../package.json
  caused by: Operation not permitted (os error 1)
```

### Cause

Hardlinks cannot cross filesystems. The most common configuration that
hits this:

- `~/.guroku` lives on your internal disk.
- Your project (and therefore `node_modules`) lives on a mounted
  external volume, network share, or Docker bind mount.

The linker is supposed to detect this and fall back to copying.

### Fix

The linker should fall back to copying automatically. If it doesn't —
i.e. you see the error above instead of a slightly slower install —
that's a bug. File an issue with:

- output of `mount` (or `wmic logicaldisk get` on Windows),
- the path of `~/.guroku`,
- the path of your project.

As a workaround you can move `~/.guroku` onto the same filesystem as
your project by setting `GUROKU_HOME`:

```sh
export GUROKU_HOME=/Volumes/Work/.guroku
```

---

## Tools that can't resolve through symlinks

### Symptom

A bundler, type-checker, or test runner can't find a module that
guroku has clearly installed. The file exists at
`node_modules/X/index.js`, but the tool reports "cannot find module
X".

### Cause

The tool is following the symlink and getting confused — often
because it normalizes paths through `realpath` and then can't match
them against its own module-resolution rules.

In 2026 essentially every actively-maintained tool handles symlinked
`node_modules` correctly (pnpm has been the default in much of the
ecosystem for years). The tools that still trip on it are mostly
unmaintained legacy bundlers.

### Fix

Two flags are planned for v0.4 to make this easy:

- `node-linker=hoisted` (in `.gurokurc`) — installs a flat,
  npm-style `node_modules`.
- `--use-flat-layout` on the command line — same thing, per-invocation.

**Neither flag exists in v0.3.** If you genuinely cannot use the
strict layout today, your options are:

- Update or replace the broken tool. This is the right answer in 99%
  of cases.
- Use npm or pnpm for that project until v0.4 ships.

---

## Slow first install

### Symptom

The first `guroku install` in a new project (or on a new machine)
feels slower than you expected. Subsequent installs are fast.

### Cause

This is working as designed. The first install is a **cold cache**
install: every tarball has to be fetched from the registry, verified,
unpacked into the content-addressable store, and linked into
`node_modules`. The bottleneck is the network, not guroku.

After that, the CAS at `~/.guroku/cas` is warm. Re-installing the
same versions in any project on the same machine reuses the unpacked
files via hardlinks/symlinks, and is dramatically faster.

### Fix

Nothing to fix. If first installs are unusably slow on your network,
file an issue with timings — there's still room to parallelize fetches
more aggressively.

---

## Stale metadata cache

### Symptom

A package was just published to the registry, you can see it on
`npmjs.com`, but `guroku install` insists the version doesn't exist.

### Cause

guroku caches registry metadata at `~/.guroku/cache/metadata` to keep
resolution fast. The cache has a TTL, but it's longer than "I just
published two minutes ago".

### Fix

Blow away the metadata cache:

```sh
rm -rf ~/.guroku/cache/metadata
```

Then re-run install. This only deletes metadata — your CAS (the
actual unpacked packages) is untouched, so this is cheap and safe.

---

## Disk filling up

### Symptom

`~/.guroku` is multiple gigabytes and growing. `df -h` is starting to
look unhealthy.

### Cause

The CAS at `~/.guroku/cas` accumulates every version of every package
you've ever installed across every project. guroku v0.3 does not
have a real garbage collector — nothing automatically prunes versions
you no longer use.

### Fix

For background and a longer discussion, see `docs/disk-usage.md`.

The short answer for v0.3 is: when the CAS gets too big, delete it.

```sh
rm -rf ~/.guroku/cas
```

Your next install in any project will be a cold install and will
re-fetch what it needs. A real `guroku gc` is on the v0.4 roadmap.

---

## Cargo built guroku but `which guroku` shows the wrong one

### Symptom

You ran `cargo build --release` in the guroku source tree, but
`which guroku` still points at an older version (or at no version at
all):

```sh
$ which guroku
/usr/local/bin/guroku
$ guroku --version
guroku 0.2.1
```

### Cause

`cargo build --release` writes the binary to `target/release/guroku`
inside the source tree. It does **not** install the binary onto your
`$PATH`. `which guroku` is finding some older copy that was put on
`$PATH` previously (by Homebrew, by `cargo install`, by you).

### Fix

Pick one:

- Install the just-built binary onto `$PATH` system-wide:

  ```sh
  cargo install --path .
  ```

  This puts a fresh build into `~/.cargo/bin/guroku`.

- Or copy the built binary somewhere on `$PATH` yourself:

  ```sh
  cp target/release/guroku ~/.local/bin/
  ```

- Or just run it directly from the source tree:

  ```sh
  ./target/release/guroku install
  ```

To confirm you're running the version you think you're running:

```sh
guroku --version
which guroku
```

---

## Tests pass locally but fail in CI

### Symptom

Your test suite is green on your laptop. In CI, one or more tests
fail with messages about missing packages, wrong versions, or
unexpected resolution.

### Cause

Three causes, in roughly decreasing order of frequency:

1. **A test hits the network.** Locally you have a warm CAS and the
   network call quietly succeeds (or is hitting a service you happen
   to have credentials for). In CI you don't.
2. **`--cwd` is wrong.** CI runs the test command from a different
   directory than you do locally, and a test that uses guroku
   programmatically is implicitly resolving against the wrong
   `package.json`.
3. **A shared CAS between unrelated CI jobs.** If two jobs share
   `~/.guroku/cas` (because someone cached `~/.guroku` in CI without
   scoping the cache key), they can race or stomp on each other's
   installs.

### Fix

For the network case: gate the test on a `GUROKU_E2E=1` environment
variable (or similar) and run it only in jobs that opt in.

For the `--cwd` case: log the resolved `cwd` at the top of the test
and confirm it matches what you expect in CI.

For the shared-CAS case: either give each job its own
`GUROKU_HOME`, or scope the cache key to the lockfile hash so that
two jobs with different lockfiles don't share the directory.

---

## Still stuck?

If none of the above matches your symptom:

- Run with `GUROKU_LOG=debug` and re-attempt the failing command —
  the debug output usually points at the failing step.
- Check `docs/error-codes.md` for the specific error code you're
  seeing.
- Open an issue with the full command, the full output, your OS, your
  guroku version (`guroku --version`), and (if relevant) your
  `package.json` and `guroku.lock`.
