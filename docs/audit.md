# guroku audit

`guroku audit` checks the packages your project depends on against the
registry's known-vulnerability database and tells you what to do about
them. This page documents the v0.5 behavior.

## What it does

`guroku audit` reads your `guroku.lock`, collects the full `(name,
version)` set of installed packages, and asks the registry whether any
of them have published security advisories. It then prints a list of
those advisories and exits non-zero if any were found.

The lockfile is the source of truth: audit reflects what's *actually*
installed, not what's loosely requested in `package.json`. This means
you should always `guroku install` (or trust an existing lockfile)
before running audit, otherwise the result will be empty or stale.

Audit does not modify your project. It does not write to the lockfile,
the cache, or `node_modules`. It is a read-only diagnostic command.

## Quick recipe

```sh
cd my-project
guroku install
guroku audit
```

If everything is clean you'll see something like:

```
no advisories found across 84 package(s)
```

and the command will exit 0.

## Output shape

When advisories are present, the output is a flat list grouped by
package:

```
found 2 advisories across 1 package(s):
  [high] minimist@<1.2.6
         Prototype Pollution
         https://github.com/advisories/GHSA-xxxx-yyyy-zzzz
         patched: >=1.2.6
```

The fields, line by line:

- `[high]` — the severity reported by the registry. One of `low`,
  `moderate`, `high`, `critical`. Severities come straight from the
  upstream advisory; guroku does not reinterpret them.
- `minimist@<1.2.6` — the package name and the vulnerable range. If
  your installed version falls inside this range, the advisory
  applies.
- The next line is a short human-readable title.
- Then the canonical advisory URL (typically a GitHub Advisory
  Database entry).
- `patched: >=1.2.6` — the range of versions the advisory considers
  fixed. To resolve, bump the offending package into this range,
  usually via an `overrides` entry in `package.json` followed by
  `guroku install`.

When more than one advisory exists for the same package, they're
printed as separate stanzas under the same heading.

## Exit code

- `0` — no advisories were found, or the lockfile is empty.
- non-zero — at least one advisory was returned, *or* audit failed to
  reach the registry.

This makes audit straightforward to chain in CI:

```sh
guroku install --frozen-lockfile && guroku audit && deploy.sh
```

If audit fails, `deploy.sh` won't run. The same pattern works in any
shell, Makefile, or pipeline runner that respects exit codes.

Note: guroku does not currently distinguish "advisories found" from
"audit could not run" via different non-zero codes. Both are exit 1.
If you need to tell them apart in CI, capture the output and grep
for the leading `audit request failed:` prefix.

## Where the data comes from

The advisory data comes from the registry, not from guroku. Guroku
POSTs the set of installed packages to:

```
<registry>/-/npm/v1/security/advisories/bulk
```

By default `<registry>` is `https://registry.npmjs.org`. The endpoint
is the same one `npm audit` and `pnpm audit` use, so you should expect
the same set of advisories these tools would surface.

If your project's `.npmrc` (or one of guroku's resolution layers,
including `~/.npmrc` and per-scope overrides) sets `registry=` to
something other than npmjs.org, audit will hit *that* registry's
advisory endpoint. This is usually what you want for a private
mirror that proxies through to npmjs.org.

## Private registries

Many self-hosted registries (Artifactory, Nexus, Verdaccio in some
configurations) implement enough of the npm API to serve tarballs and
metadata, but do not proxy the `bulk` advisories endpoint. In that
case `guroku audit` will fail like this:

```
audit request failed: HTTP 404
```

The fix in v0.5 is operational: temporarily point `registry=` at
`https://registry.npmjs.org` for the audit run only. For example:

```sh
npm_config_registry=https://registry.npmjs.org guroku audit
```

This sends the bulk request to npmjs.org while leaving your normal
install flow (which still needs the private mirror for tarballs)
untouched. Note that this *does* tell npmjs.org which packages and
versions you have installed; if that's a concern, audit's value on a
fully air-gapped registry is limited until v0.5.x adds a configurable
audit registry separate from `registry=`.

## What v0.5 doesn't yet do

The v0.5 implementation is intentionally minimal. The following are
not yet supported:

- `--audit-level=<severity>` — fail only on high/critical. In v0.5,
  any advisory at any severity makes audit exit non-zero. As a
  workaround, post-process the output: if every printed line is
  `[low]` or `[moderate]`, treat the run as a soft warning.
- `--json` — machine-readable output for tools. v0.5 prints only the
  human-formatted block shown above. Parsing that block is brittle
  and not recommended for long-term use.
- `guroku audit fix` — auto-add an `overrides` entry for each
  fixable advisory. In v0.5, fixes are manual: edit
  `package.json`, re-run `guroku install`, re-run audit.
- Filtering by `dev` vs `production`. Audit currently sends every
  package in the lockfile, including dev-only graphs.

All of these are planned for v0.5.x.

## Comparison with npm/pnpm/yarn audit

If you're coming from another package manager:

- **npm audit** — uses the same registry endpoint and produces a very
  similar list. It also has `--audit-level`, `--json`, and an `npm
  audit fix` subcommand. If you ran `npm audit` against the same
  lockfile contents, you'd see roughly the same set of advisories.
- **pnpm audit** — same idea, same endpoint. Has `--audit-level` and
  `--json`.  pnpm's output groups slightly differently but the
  underlying data is identical.
- **yarn audit** — roughly equivalent. Older. Yarn's classic v1 audit
  is the closest in spirit to v0.5 of `guroku audit`: the endpoint is
  the same, the output is plain text, and there is no machine-readable
  flag in the original implementation.

If you're switching from one of these tools, the rule of thumb is:
the *findings* should match, but the *flags* don't carry over yet.

## Diagnostics

When audit doesn't behave the way you expect, two things help.

First, raise the log level:

```sh
GUROKU_LOG=debug guroku audit
```

This prints, among other things, the request URL guroku is hitting.
That tells you immediately whether `.npmrc` overrides took effect, and
whether you're talking to the registry you think you are.

Second, inspect cached metadata. Guroku stores per-package metadata
under `~/.guroku/cache/metadata/`. When the registry attaches
advisory data to a specific version (some private registries do
this), it ends up here:

```sh
cat ~/.guroku/cache/metadata/<pkg>.json | jq '.versions["1.0.0"].vulnerabilities'
```

If this returns `null` for every package, it's normal: the npmjs.org
metadata format does not embed vulnerabilities, the bulk endpoint
serves them separately. The `jq` query is mostly useful when you're
auditing against a private registry that *does* embed them.

## CI cadence

The set of installed packages changes when you push a commit. The set
of *known* vulnerabilities changes continuously, independent of your
code. A package you installed and audited cleanly last month may pick
up a new advisory tomorrow.

For this reason it's a good idea to run audit on a schedule, not just
on push. A weekly cron is a sensible default:

```yaml
on:
  schedule:
    - cron: '0 9 * * 1'   # Mondays at 09:00 UTC
```

Even if your code doesn't change, advisory data does. A scheduled job
catches the difference. The repo ships
`.github/workflows/audit-cron.yml` as a template; copy it into your
own project and adjust the schedule and notification step (Slack,
email, an issue filer) to taste.

If you're not on GitHub Actions, the same pattern works on any
scheduler: GitLab CI's `schedules`, CircleCI's scheduled pipelines,
plain cron on a build host, etc. The only requirement is a runner
that can `guroku install --frozen-lockfile && guroku audit`.

## Limitations

A few things worth knowing about the v0.5 implementation:

- **`file:` and `git:` deps are sent.** The lockfile contains every
  resolved entry, and audit doesn't filter by source type. So a
  `file:../local-lib` or `git+https://...` dep gets included in the
  bulk request. The registry has no advisories for these
  ("advisories for `file:..`" is not a meaningful question), so they
  are silently absent from the output. They do not cause errors, but
  they do mean audit has no opinion about your local or git-pinned
  code. Treat those graphs separately.
- **Rate limits.** The advisories endpoint may rate-limit aggressive
  callers, especially behind a shared NAT (a CI provider's egress
  IPs). Audits don't need to run more often than once per push, plus
  the weekly cron above. If you do hit a limit, you'll see a 429 in
  the failure message; back off and retry.
- **Lockfile staleness.** Audit only looks at `guroku.lock`. If the
  lockfile is out of date with `package.json`, audit reflects the
  lockfile, not the manifest. Use `--frozen-lockfile` in CI to make
  this explicit.
- **No offline mode.** Audit always makes a network request. There
  is no cached/offline audit in v0.5; if the registry is unreachable,
  audit fails. This is by design — stale advisory data is worse than
  no data — but it does mean audit cannot run on a fully air-gapped
  build.

For the planned roadmap that addresses several of these (severity
filtering, JSON output, audit fix, dev/prod split), see the v0.5.x
section above.
