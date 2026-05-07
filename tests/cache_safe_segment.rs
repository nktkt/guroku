use guroku::cache::safe_segment;

#[test]
fn unscoped_unchanged() {
    assert_eq!(safe_segment("lodash"), "lodash");
}

#[test]
fn scoped_replaces_single_slash() {
    assert_eq!(safe_segment("@types/node"), "@types+node");
}

#[test]
fn multiple_slashes_all_replaced() {
    assert_eq!(safe_segment("a/b/c/d"), "a+b+c+d");
}

#[test]
fn empty_string_passes_through() {
    assert_eq!(safe_segment(""), "");
}

#[test]
fn leading_slash_replaced() {
    assert_eq!(safe_segment("/leading"), "+leading");
}

#[test]
fn trailing_slash_replaced() {
    assert_eq!(safe_segment("trailing/"), "trailing+");
}

#[test]
fn unicode_unchanged() {
    assert_eq!(safe_segment("漢字"), "漢字");
}

#[test]
fn dots_unchanged() {
    assert_eq!(safe_segment("a.b"), "a.b");
}

#[test]
fn at_sign_alone_unchanged() {
    assert_eq!(safe_segment("@"), "@");
}
