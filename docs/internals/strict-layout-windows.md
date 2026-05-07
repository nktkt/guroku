# Strict Layout on Windows

This document covers Windows-specific gotchas in v0.3's strict layout. The
strict layout is the same shape on every platform, but the primitives it
relies on (symlinks, long paths) behave differently on Windows. Read this
before debugging "it works on my Mac" reports.

## TL;DR

The strict layout requires symlinks. On Windows, creating symlinks normally
requires either:

- Developer Mode (Windows 10 build 14972 and later), or
- a process running with administrator privileges.

Without one of those, every `guroku install` will fail partway through with
an `Io` error from the linker stage. The most common signal is `os error
1314` ("A required privilege is not held by the client").

If you are a Windows user setting up guroku for the first time, enable
Developer Mode and forget about it. Everything else in this document is
context.

## Why symlinks are gated on Windows

This is a historical artifact of the Windows ACL model. For most of
Windows' lifetime, creating a symlink required the
`SeCreateSymbolicLinkPrivilege`, which by default was only granted to
administrators. The reasoning at the time was that symlinks could be used
to mount confused-deputy attacks against services that walked the
filesystem on behalf of higher-privileged users.

Microsoft loosened this in Windows 10 build 14972 (late 2016): when the
machine is in Developer Mode, processes running at medium integrity can
create symlinks without the privilege. This is what every Unix-leaning
toolchain on Windows ends up depending on, including pnpm, bun, and now
guroku.

We do not try to escalate privileges, prompt for UAC, or write any kind of
"helper service". The user opts in once via Developer Mode, and the rest
just works.

## What guroku does on Windows

The linker stage (`crates/guroku-link/src/lib.rs`) uses the standard
library's Windows-specific symlink APIs:

- `std::os::windows::fs::symlink_file` for files.
- `std::os::windows::fs::symlink_dir` for directories.

We have to pick the right one up front because Windows distinguishes the
two at creation time, unlike POSIX where a single `symlink(2)` covers both.
The branch is straightforward:

```rust
if target.is_file() {
    std::os::windows::fs::symlink_file(&target, &link)?;
} else {
    std::os::windows::fs::symlink_dir(&target, &link)?;
}
```

If the target does not exist yet at the time we link (rare, but possible
during partial installs), we treat it as a directory link. The linker
later validates the layout, so a wrong guess will surface as a layout
error rather than silent corruption.

## Junctions are not symlinks

Windows has a second mechanism that looks like symlinks at first glance:
NTFS junctions, created via `mklink /J` or the `CreateSymbolicLink` API
with a different flag. Junctions:

- only point at directories, never files,
- only work on the local filesystem (no network paths),
- do not require any special privilege to create.

That last point is tempting. We could side-step the Developer Mode
requirement entirely by using junctions for the strict layout's
`node_modules/<name> -> .guroku/<name>@<version>/node_modules/<name>`
edges, since those are all directories.

We deliberately do not. Reasons:

- Junctions resolve at the kernel level in a way that confuses some
  Node.js tooling, particularly anything that does its own `realpath`
  walking.
- Junctions cannot point at directories on a different volume, which
  breaks the case where the CAS lives on a different drive from the
  project (a common pnpm/guroku setup on Windows).
- Mixing junctions for directories and symlinks for files would mean two
  different fix-up paths during uninstall and `guroku gc`.

So the rule is: real symlinks everywhere, gated on Developer Mode, no
junctions.

## Hardlinks on NTFS

The CAS-into-package-dir step uses `CreateHardLink`, exposed in Rust via
`std::fs::hard_link`. Hardlinks on NTFS work for any user on any file on
the same volume; no privilege is required. This is why CAS materialization
is unaffected by the Developer Mode situation.

Concretely, the install pipeline on Windows is:

1. Fetch tarballs into the CAS (`~/.guroku/store/<hash>/...`). Plain file
   I/O, no privileges needed.
2. Hardlink CAS files into each package's content directory under
   `.guroku/<name>@<version>/node_modules/<name>`. Works without
   privileges as long as both paths are on the same NTFS volume.
3. Symlink the strict-layout edges. **This** is the step that requires
   Developer Mode.

If your CAS is on a different volume from your project (e.g. CAS on `D:`,
project on `C:`), step 2 falls back to a copy. That is slower, but it
still does not require any privilege. The bottleneck is the symlink layer
in step 3.

## Enabling Developer Mode

There are two equivalent ways. Both require an administrator only for the
initial flip; once Developer Mode is on, everyday `guroku install` calls
run as a normal user.

GUI:

1. Settings -> Update & Security -> For developers.
2. Toggle Developer Mode on.
3. Accept the elevation prompt.

Registry, from an elevated shell:

```bat
reg add HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock /t REG_DWORD /f /v AllowDevelopmentWithoutDevLicense /d 1
```

You do not need to reboot. Newly spawned processes will pick up the
relaxed symlink policy immediately. Existing shells need to be restarted
(or you need to log out and back in) because the access token is captured
at logon.

## WSL is fine

If you are running guroku inside WSL2 against a Linux filesystem
(`/home/<you>/...`, or anywhere under `/mnt/wsl/...`), none of this
matters. WSL uses native POSIX symlinks at the kernel level, and the
Windows symlink privilege is not consulted.

The one caveat is running WSL guroku against a Windows-mounted path under
`/mnt/c/`. That goes through the 9P translation layer, which does honor
the Windows ACL model, and you will hit the same Developer Mode
requirement. Don't do that for serious projects; the performance is also
poor. Keep `node_modules` on the Linux side.

## Diagnostics on Windows

A few quick checks when something goes wrong:

```cmd
dir node_modules
```

In a strict-layout install, top-level packages show up as `<SYMLINKD>`
entries pointing into `.guroku\<name>@<version>\node_modules\<name>`. If
they show up as plain directories, the linker stage either failed
silently or the layout was materialized by a different tool (npm, yarn).

```cmd
mklink /?
```

Documents the underlying API. Useful sanity check that `mklink` is
available at all on the box.

If the install fails with `os error 1314`, the message is "A required
privilege is not held by the client" and it always means: not running as
admin, and Developer Mode is off. Enable Developer Mode and retry. If you
see `os error 5` ("Access is denied") instead, you are usually looking at
a permission problem on the target directory rather than the symlink
privilege.

`guroku doctor` (when implemented; tracked on the v0.4 roadmap) will
detect the Developer Mode state and print a one-line remediation hint
before the linker even starts.

## Path-length limits

Windows paths default to 260 characters (`MAX_PATH`). The strict layout
expands paths aggressively:

```
C:\Users\<user>\projects\<proj>\.guroku\<name>@<version>\node_modules\<name>\...
```

For deeply nested transitive dependencies, you can blow past 260
characters before getting to the actual file you care about. The failure
mode is an `Io` error with `os error 3` ("The system cannot find the path
specified") or `os error 206` ("The filename or extension is too long"),
which is surprising the first time you see it because the file is right
there.

The fix is to enable long-path support, which is two steps:

```bat
reg add HKLM\SYSTEM\CurrentControlSet\Control\FileSystem /t REG_DWORD /f /v LongPathsEnabled /d 1
```

and ensure the consuming application has a manifest opt-in
(`<longPathAware>true</longPathAware>`). The Rust standard library on
recent toolchains opts in by default, so guroku itself is fine. The
problem is the downstream consumer, usually `node.exe`. Check that your
Node.js installation is recent enough to be long-path aware; the LTS
builds in the 18+ line are.

For serious projects on Windows, treat long-path support as mandatory,
not optional.

## Future work

A "Windows-friendly" mode that falls back to copy-everywhere when
symlinks are not allowed is tracked on the v0.4 roadmap. The shape we
have in mind:

- Detect at install time whether symlink creation works (probe a temp
  symlink in the project directory).
- If it does not, switch the linker to a copy strategy that preserves the
  strict layout's directory shape but materializes each file as a
  hardlink (within-volume) or copy (cross-volume).
- Record the chosen strategy in the lockfile metadata so subsequent
  installs are consistent.

The downside is that copy-mode installs lose the disk-savings property of
the CAS for the project tree (the CAS itself is still deduplicated, but
each project pays full price for its `node_modules`). It also makes
`guroku gc` more conservative because we cannot rely on link counts to
prove a CAS entry is unreferenced.

Until that lands, the recommended path is: enable Developer Mode, enable
long paths, and treat WSL2 as a first-class option for anyone who finds
the Windows-native story rough.
