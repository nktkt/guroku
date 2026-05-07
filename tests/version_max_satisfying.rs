use guroku::version::{max_satisfying, parse_range};

fn pick(range: &str, versions: &[&str]) -> Option<String> {
    max_satisfying(versions.iter().copied(), &parse_range(range).unwrap()).map(|v| v.to_string())
}

#[test]
fn picks_highest_in_caret_range() {
    let got = pick("^1", &["1.0.0", "1.2.0", "1.5.3", "2.0.0"]);
    assert_eq!(got.as_deref(), Some("1.5.3"));
}

#[test]
fn picks_exact_when_only_one_matches() {
    let got = pick("2.0.0", &["1.0.0", "2.0.0", "3.0.0"]);
    assert_eq!(got.as_deref(), Some("2.0.0"));
}

#[test]
fn returns_none_when_nothing_matches() {
    let got = pick("^2", &["1.0.0"]);
    assert_eq!(got, None);
}

#[test]
fn skips_garbage_candidates() {
    let got = pick("^1", &["not-a-version", "1.0.0", "also-bad", "1.5.0"]);
    assert_eq!(got.as_deref(), Some("1.5.0"));
}

#[test]
fn empty_candidates_returns_none() {
    let got = pick("*", &[]);
    assert_eq!(got, None);
}

#[test]
fn picks_pre_release_when_range_has_pre_release() {
    let got = pick(
        ">=1.0.0-alpha",
        &["1.0.0-alpha.1", "1.0.0-beta.1", "1.0.0-beta.2"],
    );
    assert_eq!(got.as_deref(), Some("1.0.0-beta.2"));
}

#[test]
fn tilde_picks_highest_patch() {
    let got = pick("~1.2.0", &["1.2.0", "1.2.5", "1.3.0"]);
    assert_eq!(got.as_deref(), Some("1.2.5"));
}
