use guroku::npmrc::parse;

#[test]
fn simple_key_value() {
    let m = parse("registry=https://registry.npmjs.org");
    assert_eq!(m["registry"], "https://registry.npmjs.org");
}

#[test]
fn whitespace_around_equals_trimmed() {
    let m = parse("  registry  =  https://x  ");
    assert_eq!(m["registry"], "https://x");
}

#[test]
fn quoted_value_unquoted() {
    let m = parse(r#"registry="https://x""#);
    assert_eq!(m["registry"], "https://x");
}

#[test]
fn comments_skipped_semicolon() {
    let m = parse("; comment\nregistry=x");
    assert_eq!(m.len(), 1);
    assert_eq!(m["registry"], "x");
}

#[test]
fn comments_skipped_hash() {
    let m = parse("# comment\nregistry=x");
    assert_eq!(m.len(), 1);
    assert_eq!(m["registry"], "x");
}

#[test]
fn blank_lines_ignored() {
    let m = parse("\n\nregistry=x\n\n");
    assert_eq!(m.len(), 1);
    assert_eq!(m["registry"], "x");
}

#[test]
fn last_value_wins_on_duplicate_keys() {
    let m = parse("registry=a\nregistry=b");
    assert_eq!(m["registry"], "b");
}

#[test]
fn scoped_registry_key_preserved() {
    let m = parse("@types:registry=https://x");
    assert_eq!(m["@types:registry"], "https://x");
}

#[test]
fn auth_token_key_preserved() {
    let m = parse("//registry.npmjs.org/:_authToken=secret");
    assert!(m.contains_key("//registry.npmjs.org/:_authToken"));
}

#[test]
fn empty_input_returns_empty_map() {
    assert!(parse("").is_empty());
}
