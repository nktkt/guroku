# `--frozen-lockfile` runbook

This example is documentation only. There is no `package.json` next to this
README; the file is a runbook for diagnosing and resolving `--frozen-lockfile`
errors emitted by `guroku`.

## What `--frozen-lockfile` is for

`--frozen-lockfile` is intended for CI invocations of `guroku install`. With the
flag set, `guroku` refuses to update `guroku.lock` under any circumstance. If
the lockfile and `package.json` have drifted, `guroku` exits non-zero with the
message:

```
lockfile is out of date with package.json
```

The intent is to guarantee that CI installs exactly the dependency graph that
was committed, and to fail loudly if someone forgot to commit a refreshed
lockfile.

## When it fires

There are three common cases where `--frozen-lockfile` will reject the install:

a. **No lockfile exists yet.** `guroku.lock` is missing from the working
   directory. `--frozen-lockfile` cannot generate one; it can only verify an
   existing file.

b. **A new dep was added to `package.json`** but `guroku install` (without
   `--frozen-lockfile`) was not run before commit. The lockfile has no entry
   covering the new root, so the check fails.

c. **A dep was removed from `package.json`** but the lockfile still references
   it. The stale root in the lockfile no longer corresponds to a declared
   dependency, so the check fails.

## The fix

Drop the flag once locally to regenerate the lockfile, then commit it:

```sh
guroku install
git add guroku.lock
git commit -m "Refresh guroku.lock"
```

Then push. CI will succeed on the next run because the committed lockfile now
matches `package.json`.

## CI snippet

A minimal GitHub Actions step using `--frozen-lockfile`:

```yaml
- name: Install dependencies (frozen)
  run: guroku install --frozen-lockfile
```

Place this step after your checkout and toolchain setup steps. No other flags
are required for the frozen check to apply.

## Diagnosing drift in CI

If CI reports `lockfile is out of date with package.json` but you cannot
reproduce the failure locally, the most likely culprit is your editor or a
pre-commit tool mutating `guroku.lock`. Check that:

- Your editor is not auto-formatting `guroku.lock` (e.g. reflowing JSON,
  reordering keys, or normalizing quotes).
- Your editor is not stripping the trailing newline.
- No pre-commit hook is rewriting the file.

The lockfile is parsed strictly. In particular, a missing `lockfileVersion`
field is fatal; `guroku` will not attempt to infer it.

## What it does not check

`--frozen-lockfile` is a structural check, not a semantic one. It does **not**
re-resolve the dependency graph to confirm that the lockfile is what the
resolver would produce today against the current registry state. It only
verifies that every root listed in `package.json` is covered by some matching
name in the lockfile.

That means a lockfile pinned to an old version of a dependency will pass
`--frozen-lockfile` even if the version range in `package.json` would now
resolve to something newer. v0.3 plans a stricter resolver-equivalence check
behind a separate flag.

## Related

- `examples/with-lockfile/` — a worked example with a committed lockfile.
- `docs/lockfile-format.md` — the on-disk format and parser rules for
  `guroku.lock`.
