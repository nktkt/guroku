# Security Policy

## Reporting a Vulnerability

guroku is a pre-1.0 project. Please report suspected vulnerabilities **privately** via GitHub Security Advisories using the "Report a vulnerability" button on the repository's Security tab:

https://github.com/nktkt/guroku/security/advisories/new

Do **not** open public issues, pull requests, or discussions for security bugs.

## Scope

In scope:

- The guroku CLI and library code in this repository.
- Tarball extraction safety (path handling, symlink handling, entry filtering).
- Integrity verification of downloaded artifacts.
- Handling of registry responses (parsing, validation, trust boundaries).

Out of scope:

- Vulnerabilities in upstream npm packages installed by guroku.
- Vulnerabilities in Node.js itself.
- Vulnerabilities in the npm registry service.

Issues in out-of-scope components should be reported to the relevant upstream project.

## Supported Versions

guroku is pre-1.0. Only the latest tagged release receives security fixes. Older releases will not be patched; users should upgrade.

| Version | Supported          |
| ------- | ------------------ |
| v0.1    | Yes                |
| < v0.1  | No                 |

## Response Expectations

- Best-effort acknowledgement of reports within 7 days.
- As a pre-1.0 project, there is no formal SLA for triage, fix, or disclosure.
- Maintainers will coordinate disclosure timing with the reporter, including any embargo period needed to prepare and release a fix.

## Hardening

v0.1 already implements:

- SHA-512 verification of every downloaded tarball before extraction.
- Path-traversal rejection: tarball entries whose paths contain `..` are refused.
- Leading `package/` segment is stripped from tarball entries; the tar's claimed filename is otherwise not trusted for placement.

Not yet in place:

- Lockfile attestation.
- Signed packages.
- Sandboxed lifecycle scripts.

These gaps are known and tracked; reports that demonstrate concrete exploitation paths within the current threat model are still welcome.

## Credit

Security reporters are acknowledged in `CHANGELOG.md` for the release containing the fix, unless the reporter prefers to remain anonymous. Indicate your preference in the initial report.
