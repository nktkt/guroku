# `.npmrc` reader

This document describes how guroku v0.4 discovers, parses, and consumes
`.npmrc` files. The reader lives in the `npmrc` module and is intentionally
minimal: it covers just enough of the npm config surface area for the
registry client to find the right index, and treats everything else as
opaque key/value data.

## What lives in `.npmrc`

`.npmrc` is the standard npm-ecosystem config file. Each line is either
a comment, blank, or a `key=value` pair. The keys guroku v0.4
understands are:

- `registry=<url>` — the default registry URL used by
  `RegistryClient` for unscoped packages.
- `<scope>:registry=<url>` — a per-scope registry override
  (for example, `@acme:registry=https://npm.acme.internal`).
  The keys are parsed and stored, but the resolver does not yet
  route per-scope requests (see "Notable npm features we don't yet
  implement" below).
- `//host/:_authToken=<token>` — a registry auth token, keyed by
  registry host. v0.4 parses these into the in-memory map but does
  not attach them to outgoing HTTP requests.

Any other key — `legacy-peer-deps`, `strict-ssl`, `prefix`, `cafile`,
`save-exact`, and so on — is preserved verbatim in the in-memory
`BTreeMap<String, String>` but ignored by the rest of the crate.
Storing unknown keys (rather than dropping them) keeps round-tripping
cheap if a future version wants to act on them, and it lets tests
inspect the full parsed state without special hooks.

## Where files live

guroku looks in two places, in this order:

1. `~/.npmrc` — the per-user global config, resolved from the
   `HOME` environment variable.
2. `<cwd>/.npmrc` — the project-local config, resolved relative to
   the directory passed to `Npmrc::discover`.

Neither file is required. A missing file is silently treated as empty;
`Npmrc::discover` only returns an error if a file exists but cannot
be read or parsed (and even then, parse errors are limited to I/O —
the line parser itself never fails, see below).

guroku v0.4 does **not** read:

- `/etc/npmrc` (the system-wide rc), nor
- the npm "builtin" rc shipped inside the npm install
  (`<prefix>/lib/node_modules/npm/npmrc`).

Both are planned for v0.5 alongside the registry-auth work.

## Lookup order

```rust
pub fn discover(cwd: &Path) -> io::Result<Npmrc> {
    let mut entries: BTreeMap<String, String> = BTreeMap::new();

    if let Some(home) = dirs::home_dir() {
        if let Some(global) = read_optional(&home.join(".npmrc"))? {
            entries.extend(global);
        }
    }
    if let Some(local) = read_optional(&cwd.join(".npmrc"))? {
        entries.extend(local);
    }

    Ok(Npmrc { entries })
}
```

The two-step `extend` is the whole precedence story:

- Global keys are inserted first.
- Project keys are extended on top.
- `BTreeMap::extend` calls `insert` for each pair, so a project-local
  key with the same name overwrites the global value.

This matches npm's documented "closer config wins" rule for the two
levels guroku currently supports.

## Parser

`parse(text)` is a 20-line line-by-line splitter. It is deliberately
not a full INI parser — npm's format is line-oriented and we only
need a small subset.

```rust
pub fn parse(text: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim().to_string();
        let value = strip_quotes(value.trim()).to_string();
        out.insert(key, value);
    }
    out
}
```

Behaviour:

- Lines starting with `;` or `#` are treated as comments and skipped.
  guroku does not support trailing comments (`key=value ; comment`);
  the entire line is treated as one value.
- Blank lines (after trimming) are skipped.
- The first `=` splits key from value. Lines with no `=` are skipped
  rather than failing — this makes the parser tolerant of stray
  text and matches npm's behaviour.
- Both sides are trimmed of ASCII whitespace.
- If the value is wrapped in matching `"..."` quotes, the quotes are
  stripped. Single quotes are left alone, again matching npm.
- On a duplicate key within one file, the **last value wins**. This
  is a side-effect of `BTreeMap::insert`, but it's the documented
  behaviour and the test suite pins it.

The parser is infallible: it returns a `BTreeMap`, not a `Result`.
All error paths in the reader come from `read_to_string`.

## Notable npm features we don't yet implement

The following are recognised npm behaviours that guroku v0.4
intentionally skips. They are listed here so callers know the gap.

- **`${VAR}` interpolation.** npm expands environment variables
  inside values (for example, `_authToken=${NPM_TOKEN}`). guroku
  stores the literal string `${NPM_TOKEN}`. If a user relies on
  this, they will see a confusing 401 once auth is wired up in v0.5.
- **`_authToken` actually being used.** The token is parsed into
  the map but never attached to a request. Registry auth lands in
  v0.5 along with `_auth` and `username`/`_password`.
- **`userconfig` and `globalconfig` overrides.** npm lets a user
  point at a non-default rc file. We always look at `~/.npmrc` and
  `<cwd>/.npmrc`.
- **The npm "builtin" rc** at `npm/etc/npmrc`. We never look inside
  the npm install directory.
- **The `legacy-peer-deps`, `strict-ssl`, `prefix`, `cafile`
  family.** These keys are preserved in the map but have no effect.
  Peer-dep handling, TLS overrides, install prefix, and custom CA
  bundles are all separate features.
- **`npm_config_*` environment variables.** See "Key collisions
  with environment variables" below.

## How `RegistryClient` consumes it

`RegistryClient::from_npmrc(cwd)` is the production constructor and
the only place outside tests that touches the reader:

```rust
impl RegistryClient {
    pub fn from_npmrc(cwd: &Path) -> io::Result<Self> {
        let rc = Npmrc::discover(cwd)?;
        Ok(RegistryClient::new(rc.registry()))
    }
}
```

`rc.registry()` returns the value of the `registry` key, falling back
to `https://registry.npmjs.org/` if the key is unset. Scoped
registries (`@scope:registry=...`) are accessible via
`rc.scoped_registry("@scope")`, but the resolver does not yet call
this — every fetch goes through the default registry. Routing
per-scope is a v0.5 task.

The intent is that anything inside `RegistryClient` that needs an rc
value reads it via the `Npmrc` handle, not by re-parsing the file.
Callers that have already built an `Npmrc` for other reasons (e.g.
the future `guroku config` subcommand) can use
`RegistryClient::with_npmrc(&Npmrc)` to avoid a second disk read.

## Test surface

Two helpers are deliberately public so tests don't have to mutate
the user's environment:

- `Npmrc::read_from(path: &Path) -> io::Result<Npmrc>` reads a
  single file and skips the discovery dance. Tests use this to point
  at a fixture in `tests/fixtures/npmrc/`.
- `parse(text: &str) -> BTreeMap<String, String>` is the in-memory
  parser. Unit tests for quoting, comments, and duplicate-key
  behaviour use this directly.

Neither helper touches `HOME` or the current directory, so tests
are safe to run in parallel and don't need a temp-home harness.

## Key collisions with environment variables

npm honours `npm_config_*` environment variables — for example,
`npm_config_registry=https://example.com` is equivalent to setting
`registry=https://example.com` in `.npmrc`, and at higher
precedence than the project-local rc. guroku v0.4 does **not** read
these variables. If a user's CI defines `npm_config_registry` and
expects guroku to honour it, guroku will silently use whatever
`.npmrc` says (or the npm default).

This is a known difference from npm. It is not a bug in v0.4 — it
is documented here so consumers can work around it by writing a
project-local `.npmrc` in their CI step. Env-var support is on the
v0.5 list.

## Reference

For the authoritative description of the file format, see npm's
documentation:

<https://docs.npmjs.com/cli/v9/configuring-npm/npmrc>

When in doubt about a corner case (quoting, comments, scope key
syntax), match npm's behaviour rather than inventing a new
convention.
