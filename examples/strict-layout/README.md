# strict-layout

An example project demonstrating the strict, pnpm-style `node_modules`
layout produced by guroku v0.3.

## What this example shows

Starting with v0.3, guroku installs packages into a content-addressed
store and links them into `node_modules` using a strict, pnpm-style
layout. Only packages that are listed as direct dependencies in
`package.json` appear at the top level of `node_modules/`. Every other
package lives under `node_modules/.guroku/<name>@<version>/` and is
only visible to the packages that actually depend on it.

This `package.json` declares two direct dependencies:

- `is-odd@^3`
- `ms@^2.1`

`is-odd` itself depends on `is-number`, so `is-number` is pulled in
transitively. The example shows how guroku keeps `is-number` reachable
to `is-odd` while hiding it from your own code.

## Try it

```sh
cd examples/strict-layout
rm -rf node_modules guroku.lock
guroku install
```

## What you should see

After install, the `node_modules` tree looks like this:

```
node_modules/
├── .guroku/
│   ├── is-number@<version>/
│   │   └── node_modules/
│   │       └── is-number/
│   ├── is-odd@<version>/
│   │   └── node_modules/
│   │       ├── is-odd/
│   │       └── is-number -> ../../is-number@<version>/node_modules/is-number
│   └── ms@<version>/
│       └── node_modules/
│           └── ms/
├── is-odd -> .guroku/is-odd@<version>/node_modules/is-odd
└── ms -> .guroku/ms@<version>/node_modules/ms
```

Note that `is-number` is NOT a top-level entry. It is only reachable
via `is-odd`'s own `node_modules/`, which is exactly how Node's module
resolution algorithm expects transitive dependencies to be exposed.

## Verify it via `ls -la`

```sh
ls -la node_modules
# is-odd  -> .guroku/...
# ms      -> .guroku/...
# .guroku  (real dir)
```

You should see two symlinks (`is-odd`, `ms`) pointing into `.guroku/`,
and `.guroku/` itself as a real directory.

## Verify hardlinks point at the CAS

Files inside `.guroku/<pkg>@<ver>/node_modules/<pkg>/` are hardlinks
back to guroku's content-addressed store (`~/.guroku/cas/`). You can
confirm by comparing inode numbers:

```sh
ls -li node_modules/.guroku/is-odd@*/node_modules/is-odd/package.json
ls -li ~/.guroku/cas/*/*/package.json | head
# Same inode number means same on-disk bytes.
```

If two paths share an inode, they are the same file on disk. That is
how guroku avoids re-downloading and re-storing duplicate package
content across every project on your machine.

## Why is-number is hidden

Only direct dependencies surface at the top level of `node_modules/`.
That means your application code can `require('is-odd')` and
`require('ms')`, but a stray `require('is-number')` will fail to
resolve. This is intentional: strict mode prevents accidental
"phantom dependency" usage, where code reaches into a transitive
dependency that is not declared in your own `package.json`.

If you actually need `is-number`, add it as a direct dependency.

## Compare with v0.2 layout

For reference, here is what guroku v0.2 produced for the same
`package.json`:

```
node_modules/
├── is-odd/
├── is-number/
└── ms/
```

Every resolved package was copied flat into the top level. That made
`require('is-number')` succeed even though `is-number` was never
declared as a direct dependency. v0.2 was simple but
phantom-dependency-prone, and it broke as soon as two packages in the
graph wanted different versions of the same dependency.

The v0.3 strict layout fixes both problems: only declared deps are
visible at the top level, and every `(name, version)` pair gets its
own isolated directory under `.guroku/`.

## Related docs

- `docs/internals/strict-layout.md` — full specification of the
  on-disk layout and the symlink rules.
- `docs/internals/cas.md` — how the content-addressed store is laid
  out and how hardlinks are created from it.
