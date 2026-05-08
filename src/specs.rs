//! Classify a `package.json` dependency spec.
//!
//! npm accepts a few shapes besides semver ranges:
//!   - `file:./local`, `file:../path` — local filesystem path
//!   - `git+https://github.com/u/r.git`, `git+ssh://git@host/...`,
//!     `github:user/repo`, `git://...`
//!   - everything else is treated as a semver range (handled by the existing
//!     `version` module).

use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DepSpec {
    /// Standard registry resolution against an npm semver range.
    Range(String),
    /// Local-filesystem path. Stored verbatim from the spec (with the
    /// `file:` prefix stripped).
    File(String),
    /// Git repository reference. Includes optional ref/branch/commit.
    Git(GitRef),
    /// `npm:<real-name>@<inner>` — install a registry package under a
    /// different local name. The `real_name` is the registry name; the
    /// inner spec drives the resolver (typically a Range).
    Alias {
        real_name: String,
        inner: Box<DepSpec>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitRef {
    pub url: String,
    /// Trailing `#<ref>` part of the spec, when present.
    pub revision: Option<String>,
}

pub fn classify(spec: &str) -> DepSpec {
    let s = spec.trim();
    if let Some(rest) = s.strip_prefix("file:") {
        return DepSpec::File(rest.to_string());
    }
    if let Some(rest) = s.strip_prefix("git+") {
        return DepSpec::Git(parse_git(rest));
    }
    if s.starts_with("git://") || s.starts_with("git@") {
        return DepSpec::Git(parse_git(s));
    }
    if let Some(rest) = s.strip_prefix("github:") {
        let url = format!("https://github.com/{rest}");
        return DepSpec::Git(parse_git(&url));
    }
    if let Some(rest) = s.strip_prefix("npm:") {
        // `npm:<real-name>@<spec>` — alias to a registry package under a
        // different local name. We split on the LAST `@` so scoped names
        // (`@types/node@^20`) work.
        let (real_name, inner_spec) = match rest.rsplit_once('@') {
            Some((n, v)) if !n.is_empty() => (n.to_string(), v.to_string()),
            _ => (rest.to_string(), "*".to_string()),
        };
        return DepSpec::Alias {
            real_name,
            inner: Box::new(DepSpec::Range(inner_spec)),
        };
    }
    DepSpec::Range(s.to_string())
}

fn parse_git(s: &str) -> GitRef {
    if let Some((url, rev)) = s.rsplit_once('#') {
        // Tolerate `#` in URLs by checking the rev looks like a ref. If
        // it contains '/' it's likely a query/fragment, not a git ref.
        if !rev.contains('/') && !rev.is_empty() {
            return GitRef {
                url: url.to_string(),
                revision: Some(rev.to_string()),
            };
        }
    }
    GitRef {
        url: s.to_string(),
        revision: None,
    }
}

/// Convenience for the install path: turn a DepSpec back into a string
/// that can be re-parsed identically.
pub fn unparse(spec: &DepSpec) -> String {
    match spec {
        DepSpec::Range(r) => r.clone(),
        DepSpec::File(p) => format!("file:{p}"),
        DepSpec::Git(g) => match &g.revision {
            Some(r) => format!("git+{}#{r}", g.url),
            None => format!("git+{}", g.url),
        },
        DepSpec::Alias { real_name, inner } => {
            let inner_str = match &**inner {
                DepSpec::Range(r) => r.clone(),
                other => unparse(other),
            };
            format!("npm:{real_name}@{inner_str}")
        }
    }
}

/// Reject specs that v0.5 doesn't yet support beyond classification.
/// (Currently a noop; reserved for `workspace:*` etc.)
pub fn validate(_spec: &DepSpec) -> Result<()> {
    Ok(())
}
