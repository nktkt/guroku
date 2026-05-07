# `.npmrc` support

guroku v0.4 reads npm's `.npmrc` configuration files so existing projects
can point guroku at the same registry (and, eventually, the same auth) as
npm without rewriting any configuration. This page documents what guroku
v0.4 actually honours; for the parser internals see
`docs/internals/npmrc.md`.

## What `.npmrc` is

`.npmrc` is npm's per-project / per-user configuration file. It is a
plain-text file, one `key=value` pair per line, with `;` or `#` for
comments. npm reads it to discover the registry URL, auth tokens, and
hundreds of behavioural flags.

guroku v0.4 reads `.npmrc` for one purpose only: to learn which registry
URL it should fetch packages from. Other keys are parsed but mostly
ignored (see below).

## Files that get read

guroku v0.4 looks at exactly two files, in priority order:

1. `<project>/.npmrc` (highest priority) - the `.npmrc` next to the
   `package.json` that guroku is installing for.
2. `~/.npmrc` - the current user's home-directory `.npmrc`.

Keys defined in the project file override keys with the same name in the
home-directory file.

guroku v0.4 does **not** read:

- npm's "builtin" rc file (the one shipped inside the npm CLI install).
- `/etc/npmrc` (the system-wide rc).

If you rely on either of those with npm today, copy the relevant keys
into `~/.npmrc` so guroku can see them.

## Keys guroku v0.4 honours

### `registry=<url>`

The default registry URL. guroku fetches every package from this URL
unless a more specific rule applies. If neither `.npmrc` file sets
`registry`, guroku falls back to `https://registry.npmjs.org/`.

```
registry=https://corporate-mirror.internal
```

### `<scope>:registry=<url>`

A registry override for a single npm scope, e.g.:

```
@acme:registry=https://npm.acme.com
```

guroku v0.4 parses this key and stores it in the in-memory config map,
**but the resolver does not yet route per-scope traffic**. Every request
in v0.4 still goes to the default `registry`. Per-scope routing is
scheduled for v0.5.

You can keep these lines in your `.npmrc` today; they will start taking
effect automatically once you upgrade to v0.5.

### `//<host>/:_authToken=<token>`

A bearer token to be sent to a specific registry host, e.g.:

```
//npm.acme.com/:_authToken=hunter2
```

guroku v0.4 reads this into the in-memory map, **but does not yet attach
the token to outgoing HTTP requests**. Auth-aware fetching lands in v0.5.

If your private registry requires a token, v0.4 will not be able to
install from it. Stay on npm/your existing tool for those packages until
v0.5 ships.

## Keys we IGNORE

Many common npm keys are parsed (so the file does not error out) and
stored in the in-memory map, but guroku v0.4 makes no use of them. Among
others:

- `legacy-peer-deps`
- `strict-ssl`
- `prefix`
- `cafile`
- `cache`
- `fund`
- `audit`
- `save-exact`
- `engine-strict`

guroku v0.4 does **not** emit a warning when it sees these keys. The
file is accepted silently. This is documented behaviour, not a bug:
npmrc files in the wild are full of keys guroku does not implement, and
warning on every one would be noisy. If a key you care about is missing,
file a ticket.

## Worked example

Given a single home-directory file:

```
; ~/.npmrc
registry=https://corporate-mirror.internal
@acme:registry=https://npm.acme.com
//npm.acme.com/:_authToken=hunter2
```

In guroku v0.4:

- Every install of a public package (e.g. `lodash`, `react`) fetches
  from `https://corporate-mirror.internal`. Good - that is what the
  user asked for.
- Installs of `@acme/*` packages **also** fetch from
  `https://corporate-mirror.internal`, **not** from `npm.acme.com`. The
  per-scope override is parsed but not yet honoured by the resolver
  (see v0.5).
- The auth token for `npm.acme.com` is parsed and held in memory but
  not attached to any request. If `corporate-mirror.internal` happens
  to proxy `@acme/*` from `npm.acme.com` and does so without auth, the
  install works. If not, it fails with a 401/404 from the mirror.

In other words: in v0.4, `.npmrc` is effectively a one-key file
(`registry`). The other supported keys are wired up but inert.

## Comments and quoting

The parser is forgiving in the same ways npm's parser is:

- Lines beginning with `;` or `#` are comments and are ignored.
- Inline comments are **not** stripped - a `;` halfway through a value
  is part of the value. (npm behaves the same way.)
- Values may be wrapped in `"..."`. The surrounding quotes are
  stripped; the inner text is the value.
- Whitespace around `=` is trimmed from both sides. Trailing whitespace
  on the value is also trimmed.
- Blank lines are ignored.

Examples:

```
# this is a comment
; this is also a comment

registry = https://example.com/        # leading/trailing space trimmed
some-key="value with spaces"           # quotes stripped
```

## Environment-variable interpolation

npm expands `${VAR}` references inside `.npmrc` values against the
process environment. **guroku v0.4 does not.** A line like:

```
//npm.acme.com/:_authToken=${ACME_TOKEN}
```

is read literally - the value is the seven-character string
`${ACME_TOKEN}`, not the contents of the environment variable.

Interpolation is tracked for a v0.4.x point release. For now, write the
literal value into the file, or generate the file at build time.

## `npm_config_*` environment variables

npm also accepts configuration via environment variables of the form
`npm_config_<key>` (e.g. `npm_config_registry=...`). **guroku v0.4 does
not read these.** If you currently configure npm through environment
variables, mirror the values into `~/.npmrc` for guroku.

This may change in a later release; for v0.4, `.npmrc` files are the
only configuration channel.

## Diagnostics

To see what guroku actually loaded from your `.npmrc` files, run with
debug logging:

```
GUROKU_LOG=debug guroku install
```

The debug output includes:

- The path of each `.npmrc` file that was read (or skipped because it
  did not exist).
- The resolved default registry URL.
- Every scoped registry override that was discovered (even though they
  are not yet routed).
- The set of hosts for which an `_authToken` was found (token values
  are redacted in the log).

If a registry override is not taking effect, this is the first place to
look.

## FAQ

**Why isn't my private registry token being sent?**

guroku v0.4 reads `_authToken` entries but does not yet attach them to
outgoing HTTP requests. Auth lands in v0.5. Until then, private
registries that require a token cannot be used with guroku.

**Where do I put my registry override?**

Either:

- `~/.npmrc` - applies to every guroku invocation by your user.
- `<project>/.npmrc` - applies only when running guroku in that
  project.

If both files set `registry=`, the project file wins.

**Does guroku create a `.npmrc`?**

No. guroku only reads `.npmrc` files; it never writes them. There is no
`guroku config set` command in v0.4. You author `.npmrc` yourself with
a text editor, the same way you would for npm.

**Will guroku ever read `/etc/npmrc` or the builtin rc?**

Maybe. It is not on the v0.5 roadmap. If you need system-wide config,
use `~/.npmrc` for now.

**My `.npmrc` works with npm but not with guroku - what gives?**

The most common causes, in order:

1. The key you rely on is in the IGNORE list above.
2. The value uses `${VAR}` interpolation, which v0.4 does not expand.
3. The configuration comes from an `npm_config_*` env var, which v0.4
   does not read.
4. The file lives at `/etc/npmrc` or in npm's builtin rc, neither of
   which v0.4 reads.

Run with `GUROKU_LOG=debug` to see exactly which files and keys guroku
picked up.
