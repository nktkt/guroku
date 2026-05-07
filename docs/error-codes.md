# guroku error reference

This page is the single source of truth for every error guroku can emit. Each
entry shows the message template you will see on the terminal, the typical
symptom, the underlying cause, and a recommended fix. If an error is not listed
here, it is either coming from an external tool or it is a bug -- please report
it.

The error variants are defined in `src/error.rs`. Messages are reproduced here
verbatim with `{placeholders}` for the values guroku interpolates at runtime.

## Table of contents

### Filesystem and parsing

- [`io error at {path}: {source}`](#io-error-at-path-source)
- [`io error: {source}`](#io-error-source)
- [`failed to parse {path}: {source}`](#failed-to-parse-path-source)
- [`invalid json: {source}`](#invalid-json-source)

### Network and registry

- [`http error: {source}`](#http-error-source)
- [`invalid url: {source}`](#invalid-url-source)
- [`package \`{name}\` not found in registry`](#package-name-not-found-in-registry)

### Versions and resolution

- [`no matching version for \`{name}@{spec}\``](#no-matching-version-for-namespec)
- [`invalid version spec \`{spec}\` for package \`{name}\``](#invalid-version-spec-spec-for-package-name)
- [`version conflict for \`{name}\`: already chose \`{chosen}\`, but \`{requested_by}\` requires \`{requested}\``](#version-conflict-for-name-already-chose-chosen-but-requested_by-requires-requested)

### Integrity

- [`integrity check failed for \`{name}@{version}\`: {detail}`](#integrity-check-failed-for-nameversion-detail)
- [`unsupported integrity algorithm: \`{value}\``](#unsupported-integrity-algorithm-value)
- [`invalid integrity string: \`{value}\``](#invalid-integrity-string-value)
- [`tarball error: {detail}`](#tarball-error-detail)

### Lockfile

- [`lockfile version mismatch: file is v{found}, this guroku understands v{expected}`](#lockfile-version-mismatch-file-is-vfound-this-guroku-understands-vexpected)
- [`lockfile is out of date with package.json`](#lockfile-is-out-of-date-with-packagejson)

### Other

- [`could not determine cache directory`](#could-not-determine-cache-directory)
- [`{message}` (Other)](#message-other)

---

# Filesystem and parsing

These errors fire while guroku is reading or writing files on disk, or while
parsing structured data it has just read. They almost always reflect a state of
the filesystem rather than a bug in guroku itself.

## `io error at {path}: {source}`

**Symptom:** A command that touches a specific file or directory fails. The
message names the path and the underlying OS error, for example
`io error at ./node_modules/.guroku-cache/abc.tgz: Permission denied (os error 13)`.

**Cause:** guroku tried to open, create, read, or write `{path}` and the
operating system refused. Common reasons are missing parent directories, a file
locked by another process, a read-only filesystem, or insufficient permissions.

**Fix:** Read `{source}` literally -- it is the OS error and tells you what is
wrong. Check that the path exists, that you have read and write permission, and
that no other process is holding it open. On macOS, also check that the
directory is not under a protected location such as `~/Library` without Full
Disk Access.

**See also:** `docs/internals/cache-layout.md` for the on-disk layout guroku
expects.

## `io error: {source}`

**Symptom:** Same shape as the previous error, but without a path. You will see
something like `io error: Broken pipe (os error 32)`.

**Cause:** An I/O failure occurred in a context where guroku could not
attribute it to a single path -- for example while streaming bytes from the
network into a decoder, or while writing to the terminal. The bare variant is
used when wrapping a path would be misleading.

**Fix:** As above, the OS error is the actionable part. `Broken pipe` usually
means the consumer (e.g. `less`) closed early and is harmless. For other errors
re-run the command with `--verbose` to capture the full backtrace.

**See also:** [`io error at {path}: {source}`](#io-error-at-path-source).

## `failed to parse {path}: {source}`

**Symptom:** guroku stops early during install or resolve, pointing at a
manifest file: `failed to parse ./package.json: expected value at line 12 column 3`.

**Cause:** The named manifest exists and is readable, but its contents are not
valid JSON or do not match the schema guroku expects. This typically happens
when a `package.json` has a trailing comma, an unquoted key, or a stray BOM.

**Fix:** Open `{path}` and validate it as JSON (`jq . {path}` is a quick
check). Fix the syntax issue reported by `{source}`. If the file looks valid,
make sure it is UTF-8 without a BOM.

**See also:** `docs/internals/manifest.md` for the manifest fields guroku
reads.

## `invalid json: {source}`

**Symptom:** A non-manifest payload fails to deserialize. Most often seen when
talking to the registry: `invalid json: missing field "versions" at line 1 column 87`.

**Cause:** guroku received bytes that were supposed to be JSON but either are
not, or do not match the expected shape. The registry response, a lockfile, or
a cached metadata blob is corrupt or unexpected.

**Fix:** If the source is a registry, retry -- transient corruption is rare
but possible behind some proxies. If the error is reproducible, clear the
metadata cache (`guroku cache clean --metadata`) and try again. If you set a
custom registry, confirm it actually speaks the npm registry protocol.

**See also:** `docs/internals/registry-protocol.md`.

---

# Network and registry

These errors are raised when guroku is talking to a registry over HTTP, or
when constructing the URLs it needs to do so.

## `http error: {source}`

**Symptom:** Install or fetch fails with a message such as
`http error: error sending request for url (https://registry.npmjs.org/...): connection closed before message completed`.

**Cause:** The HTTP client (reqwest) reported a transport-level error: DNS
resolution failed, the TLS handshake failed, the connection dropped, the
server returned an error status, or a timeout elapsed.

**Fix:** Confirm you have network access (`curl -I https://registry.npmjs.org`).
If you are behind a proxy, set `HTTPS_PROXY` and `HTTP_PROXY` in your
environment before running guroku. For TLS errors, ensure your system trust
store is up to date. For 4xx and 5xx responses, the body is included in
`{source}` -- read it carefully; the registry usually explains what it
rejected.

**See also:** `docs/cli-reference.md` for proxy and registry configuration
flags.

## `invalid url: {source}`

**Symptom:** guroku refuses to start a fetch:
`invalid url: relative URL without a base`.

**Cause:** A URL that guroku tried to construct or parse is not well-formed.
This can come from a malformed `registry` field in `.npmrc`, a tarball URL
embedded in registry metadata that does not parse, or a `--registry` flag with
a typo.

**Fix:** Check the registry URL you configured. It must include a scheme
(`https://`) and a host. If the offending URL is in registry metadata, the
package itself is at fault -- file an issue with the package author.

**See also:** `docs/cli-reference.md#registry`.

## `package \`{name}\` not found in registry`

**Symptom:** A specific dependency cannot be installed:
`package \`leftpadx\` not found in registry`.

**Cause:** The registry returned 404 for `{name}`. The package does not exist
under that exact name on the configured registry.

**Fix:** Check spelling and scope. Scoped packages must include the leading
`@` (e.g. `@types/node`). If the package is private, make sure you are
authenticated (`guroku login`) and pointing at the right registry. If you
recently published, allow a few seconds for replication.

**See also:** `docs/cli-reference.md#login`.

---

# Versions and resolution

These errors describe situations where guroku could find the package but
could not pick a version that satisfies all the constraints in your project.

## `no matching version for \`{name}@{spec}\``

**Symptom:**
`no matching version for \`react@^99.0.0\``.

**Cause:** The package exists, but no published version satisfies `{spec}`.
Either the spec is too tight, or the version you want has been unpublished or
deprecated.

**Fix:** Run `guroku view {name} versions` to list what is actually
available, then loosen or correct the spec in your `package.json`. If the
package was unpublished, pick a different version or fork.

**See also:** `docs/internals/semver.md` for how guroku interprets ranges.

## `invalid version spec \`{spec}\` for package \`{name}\``

**Symptom:**
`invalid version spec \`~> 1.2\` for package \`lodash\``.

**Cause:** `{spec}` is not a valid npm semver range. guroku follows the same
syntax as npm: `^1.2.3`, `~1.2.3`, `1.2.x`, `>=1.0.0 <2.0.0`, exact versions,
dist-tags such as `latest`, and git/file specifiers. Anything else is
rejected up front.

**Fix:** Replace the spec with valid semver. If you intended a Ruby-style
range like `~>`, the npm equivalent is `~`. Run `guroku why {name}` to find
which dependency edge introduced the bad spec.

**See also:** `docs/internals/semver.md`.

## `version conflict for \`{name}\`: already chose \`{chosen}\`, but \`{requested_by}\` requires \`{requested}\``

**Symptom:**
`version conflict for \`react\`: already chose \`18.2.0\`, but \`some-plugin@1.0.0\` requires \`^17.0.0\``.

**Cause:** During resolution, guroku selected `{chosen}` for a top-level or
hoisted slot, then later visited `{requested_by}` whose constraint is
incompatible. With strict resolution enabled, guroku refuses to silently
duplicate the package.

**Fix:** Either align constraints (upgrade or downgrade one side), or allow
duplication by removing `--strict-peer-deps` / `resolutions.strict` from your
config. You can also pin `{name}` explicitly via the `resolutions` field in
`package.json` to force one version tree-wide.

**See also:** `docs/internals/resolver.md` and `docs/internals/semver.md`.

---

# Integrity

These errors come from guroku's integrity layer, which verifies that the
bytes it puts on disk match the hash recorded in the lockfile or registry
metadata. Treat any of these as potentially security-relevant.

## `integrity check failed for \`{name}@{version}\`: {detail}`

**Symptom:**
`integrity check failed for \`left-pad@1.3.0\`: expected sha512-AAA..., got sha512-BBB...`.

**Cause:** guroku downloaded the tarball for `{name}@{version}` and computed
its hash, but the result did not match the expected integrity string from the
lockfile or registry. This means the tarball changed in transit, was
tampered with, or the lockfile is stale.

**Fix:** Do not bypass this error. First, retry once -- a corrupt download is
possible. If it still fails, clear the cache (`guroku cache clean`) and run
`guroku install` again. If the error persists, compare the lockfile entry
with the registry's published integrity (`guroku view {name}@{version} dist.integrity`).
If they disagree, investigate before proceeding.

**See also:** `docs/internals/integrity.md`.

## `unsupported integrity algorithm: \`{value}\``

**Symptom:**
`unsupported integrity algorithm: \`md5\``.

**Cause:** A lockfile or registry record used an integrity algorithm that
guroku will not accept. guroku currently supports `sha256`, `sha384`, and
`sha512`. Older algorithms (`sha1`, `md5`) are deliberately rejected.

**Fix:** Regenerate the lockfile (`rm guroku.lock && guroku install`) so the
entries are rewritten with a supported algorithm. If the registry serves only
weak hashes, switch to a registry that publishes `sha512`.

**See also:** `docs/internals/integrity.md#supported-algorithms`.

## `invalid integrity string: \`{value}\``

**Symptom:**
`invalid integrity string: \`sha512AAAA\``.

**Cause:** The integrity string did not match the SRI format
(`<algorithm>-<base64-digest>`). Either the dash is missing, the digest is not
valid base64, or the field is empty.

**Fix:** Open the lockfile and look at the offending entry. If you are
hand-editing the lockfile, stop and let guroku rewrite it. If the bad value
came from registry metadata, file an issue with the registry operator.

**See also:** `docs/internals/integrity.md`.

## `tarball error: {detail}`

**Symptom:**
`tarball error: unexpected end of file while reading entry header`.

**Cause:** The tarball downloaded for a package is malformed -- truncated,
not gzip-compressed, contains entries with absolute paths, or otherwise
violates guroku's safe-extract rules.

**Fix:** Clear the cache and retry. If the same package keeps failing,
download the tarball manually with `curl` and inspect it with `tar -tzf`. A
genuinely broken tarball is a registry or publisher problem; report it to
the package author.

**See also:** `docs/internals/integrity.md` and `docs/internals/tarball.md`.

---

# Lockfile

These errors describe disagreements between `guroku.lock` and the rest of
the world.

## `lockfile version mismatch: file is v{found}, this guroku understands v{expected}`

**Symptom:**
`lockfile version mismatch: file is v3, this guroku understands v2`.

**Cause:** The lockfile on disk was written by a different version of guroku
that uses a newer (or older) on-disk format. guroku refuses to silently
downgrade the file because that would lose information.

**Fix:** If `{found}` is greater than `{expected}`, upgrade your local
guroku to a version that supports v`{found}`. If `{found}` is less than
`{expected}`, run `guroku install` to migrate the file in place -- guroku
will rewrite it in the current format.

**See also:** `docs/lockfile-format.md` and `docs/migration/`.

## `lockfile is out of date with package.json`

**Symptom:**
`lockfile is out of date with \`package.json\` (run \`guroku install\` without --frozen-lockfile to refresh)`.

**Cause:** You ran a command with `--frozen-lockfile` (or in CI, where it is
the default) and the lockfile no longer reflects the dependency graph implied
by `package.json`. Someone edited `package.json` without re-running install.

**Fix:** Locally, run `guroku install` without `--frozen-lockfile` -- this
recomputes the resolution and rewrites the lockfile. Commit the updated
lockfile alongside your `package.json` change. In CI, the failure is
intentional; fix it on a developer machine first.

**See also:** `docs/lockfile-format.md` and `docs/cli-reference.md#install`.

---

# Other

## `could not determine cache directory`

**Symptom:**
`could not determine cache directory`.

**Cause:** guroku could not find a sensible location for its cache. On
macOS and Linux it follows the XDG base directory spec and falls back to
`$HOME/.cache`. If neither `$XDG_CACHE_HOME` nor `$HOME` is set (common in
minimal containers), the lookup fails.

**Fix:** Export `HOME` or `XDG_CACHE_HOME` to a writable directory before
running guroku, or pass `--cache-dir` explicitly. In container images,
`ENV HOME=/root` (or another writable path) is usually enough.

**See also:** `docs/cli-reference.md#cache-dir` and
`docs/internals/cache-layout.md`.

## `{message}` (Other)

**Symptom:** A free-form error message that does not match any of the
templates above. The text comes straight from `{message}` with no prefix.

**Cause:** This is the catch-all variant. It is used for one-off conditions
that do not warrant a dedicated error code -- typically things surfaced from
plugins, scripts, or early-stage features.

**Fix:** Read the message; it is meant to be self-explanatory. If it is not,
re-run with `--verbose` to capture context, and open an issue. We treat
recurring `Other` errors as a signal that we should promote them to a
first-class variant.

**See also:** `CONTRIBUTING.md` for how to file a useful bug report.
