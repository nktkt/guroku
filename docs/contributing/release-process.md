# Release Process

This document is the maintainer checklist for cutting a guroku release. Follow
the steps in order. The flow assumes a standard release; the patch-release and
hotfix variations are described at the bottom.

guroku follows SemVer with a `0.x.y` series during pre-1.0 development. Every
`0.x` release is marked as a GitHub prerelease. We do not publish to crates.io,
so all artifacts ship through GitHub Releases.

## 1. Confirm the milestone is done

Before you start, the target version's milestone must be complete. Check both
of the following:

- Every roadmap checkbox for the target version in `README.md` is checked.
- Every roadmap checkbox for the target version in `ROADMAP.md` is checked.

If anything is unchecked, the release is not ready. Either finish the work or
move the unfinished items to the next milestone (and update both files in the
same PR) before continuing.

## 2. Run the full check locally

Run the same gauntlet CI runs, on a clean working tree, against the commit you
intend to tag:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all
cargo doc --no-deps --all-features
```

All four must pass. If `cargo doc` produces warnings, treat them as failures.
Do not skip a step because "CI will catch it" — releases are tagged from your
local commit, and a green tag pipeline does not retroactively fix a broken
artifact.

## 3. Update the version

Edit the `[package].version` field in `Cargo.toml`. Bump per SemVer:

- `0.x.0` for new milestones (resolver, CAS, lifecycle, and similar
  feature-level deliverables).
- `0.x.y` for fixes within a milestone.

Pre-1.0 we do not bump the major component; the minor component (`x`) tracks
milestones and the patch component (`y`) tracks fixes.

After editing `Cargo.toml`, run `cargo check` once so `Cargo.lock` is
regenerated with the new version pinned.

## 4. Update CHANGELOG.md

Promote the existing `[Unreleased]` section to a new version section dated
today, then add a fresh empty `[Unreleased]` heading at the top of the file.
The result should look like:

```markdown
# Changelog

## [Unreleased]

## [0.X.Y] - YYYY-MM-DD

### Added
- ...

### Changed
- ...

### Fixed
- ...
```

Do not move entries between subsections during this step; only the headings
change. If a previously-unreleased entry is wrong, fix it in a separate commit
before starting the release.

## 5. Update version-pinning docs

If the change affects user-visible behaviour, update the docs that reference
the current version or current capabilities:

- The `Status` line in `README.md` (typically a version-and-stage callout near
  the top).
- `docs/getting-started.md` if any installation snippet, command output, or
  pinned version is shown verbatim.

If the release is purely internal (refactor, dependency bump, CI tweak), this
step is a no-op — note that in the PR description so reviewers know it was
considered, not forgotten.

## 6. Update CITATION.cff

Set the following fields:

- `version` to the new version (no `v` prefix; CFF expects a bare semver
  string).
- `date-released` to today's date in `YYYY-MM-DD` form.

Leave the rest of the file alone. If the author list has changed since the
last release, that should already be a separate commit on `main`.

## 7. Commit

Stage every file touched above and commit with a single, mechanical message:

```sh
git add Cargo.toml Cargo.lock CHANGELOG.md README.md docs/getting-started.md CITATION.cff
git commit -m "Release vX.Y.Z"
```

Push to `main`. If you are not the solo maintainer, push to a branch and open
a PR; only merge once CI is green. The release tag must point at a commit that
is already on `main`.

## 8. Tag

Tag the release commit. Prefer a signed tag if you have a GPG key:

```sh
git tag -s vX.Y.Z -m "guroku vX.Y.Z"
```

If you do not have a signing key configured, fall back to an annotated tag:

```sh
git tag vX.Y.Z
```

Do not use lightweight tags. The release workflow expects an annotated or
signed tag so the message is preserved in the auto-generated notes.

## 9. Push the tag

```sh
git push origin vX.Y.Z
```

This triggers `.github/workflows/release.yml`, which:

- builds binaries for `linux-x86_64`, `linux-aarch64`, `macos-x86_64`, and
  `macos-aarch64`,
- creates a GitHub Release for the tag,
- attaches the resulting tarballs to that release.

If the workflow fails partway, do not retry by pushing a new tag with the same
name — delete the failed run's release draft, fix the underlying issue on
`main`, then either re-run the workflow against the existing tag or delete and
recreate the tag (force-pushing tags is acceptable here only because no one
has consumed it yet).

## 10. Verify the release

Open `https://github.com/nktkt/guroku/releases/tag/vX.Y.Z` and confirm:

- All four tarballs are attached (linux x86_64, linux aarch64, macos x86_64,
  macos aarch64).
- The auto-generated release notes match the CHANGELOG entry for this
  version.
- The release is marked `Pre-release` for any `0.x` version. The workflow
  should set this automatically; check it anyway.

If any of those are wrong, fix the release metadata in the GitHub UI before
moving on.

## 11. Smoke-test a binary

Download one of the published artifacts (pick the one matching your dev
machine), extract it, and run:

```sh
./guroku --version
```

The reported version must match the tag exactly. If it does not, the build
picked up a stale `Cargo.toml` — investigate before announcing.

## 12. Announce

Open a GitHub Discussion under the `Announcements` category. Link to the
release notes file (`docs/v0.X-release-notes.md`) and to the GitHub release
page. Keep the announcement short: one paragraph of context, the link, and a
call for feedback.

## 13. Yank if needed

If a release ships broken:

- Mark the GitHub release as `draft` (this hides it without deleting the tag
  or the artifacts).
- Land the fix on `main` and tag a `.1` patch release using the patch flow
  below.

Because we do not publish to crates.io, there is no `cargo yank` to issue —
the GitHub Release status is the only knob users see.

## Patch-release shortcut

A patch release follows the same flow with one substitution: step 1
("milestone is done") becomes "the bug is fixed and tested." Steps 2 through
13 are identical. Patch releases still get a CHANGELOG entry, a CITATION.cff
bump, and a signed tag.

## Hotfix branch policy

If you need to ship a fix without including in-flight work that has already
landed on `main`:

1. Branch from the previous release tag:

   ```sh
   git checkout -b hotfix/vX.Y.Z vX.Y.(Z-1)
   ```

2. Cherry-pick the fix commit (or commits) onto the hotfix branch.
3. Run the full check from step 2 on the branch.
4. Bump the version, update the changelog, update CITATION.cff, and commit
   exactly as in the standard flow.
5. Tag from the hotfix branch and push the tag. The release workflow does not
   care which branch the tag points at, only that the tag exists.
6. After the release is out, merge the hotfix branch back into `main` (or
   cherry-pick the version-bump commit forward) so the changelog and version
   metadata stay consistent.

Do not delete the hotfix branch until the next regular release has shipped;
keeping it around makes follow-up patches on the same line trivial.
