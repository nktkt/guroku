use guroku::audit::{Advisory, AuditReport};
use std::collections::BTreeMap;

#[test]
fn deserialize_full_advisory() {
    let json = r#"{
        "id": 1234,
        "title": "Path traversal",
        "severity": "high",
        "url": "https://example.com",
        "vulnerable_versions": "<1.2.3",
        "patched_versions": ">=1.2.3"
    }"#;
    let adv: Advisory = serde_json::from_str(json).expect("parse full advisory");
    assert_eq!(adv.title, "Path traversal");
    assert_eq!(adv.severity, "high");
    assert_eq!(adv.url, "https://example.com");
    assert_eq!(adv.vulnerable_versions, "<1.2.3");
    assert_eq!(adv.patched_versions, ">=1.2.3");
}

#[test]
fn deserialize_partial_advisory() {
    let json = r#"{"severity":"low"}"#;
    let adv: Advisory = serde_json::from_str(json).expect("parse partial advisory");
    assert_eq!(adv.severity, "low");
    assert_eq!(adv.title, "");
    assert_eq!(adv.url, "");
    assert_eq!(adv.vulnerable_versions, "");
    assert_eq!(adv.patched_versions, "");
    assert!(adv.id.is_null());
}

#[test]
fn deserialize_advisory_array() {
    let json = r#"[
        {"id":1,"title":"A","severity":"low","url":"u1","vulnerable_versions":"<1","patched_versions":">=1"},
        {"id":"GHSA-abcd","title":"B","severity":"high","url":"u2","vulnerable_versions":"<2","patched_versions":">=2"}
    ]"#;
    let advs: Vec<Advisory> = serde_json::from_str(json).expect("parse advisory array");
    assert_eq!(advs.len(), 2);
}

#[test]
fn audit_report_is_empty_for_empty_findings() {
    let report = AuditReport::default();
    assert!(report.is_empty());
    assert_eq!(report.count(), 0);
}

#[test]
fn audit_report_count_sums_per_name_advisories() {
    let adv = |sev: &str| Advisory {
        id: serde_json::Value::Null,
        title: String::new(),
        severity: sev.to_string(),
        url: String::new(),
        vulnerable_versions: String::new(),
        patched_versions: String::new(),
    };
    let mut findings: BTreeMap<String, Vec<Advisory>> = BTreeMap::new();
    findings.insert("a".to_string(), vec![adv("low"), adv("high")]);
    findings.insert("b".to_string(), vec![adv("moderate")]);
    let report = AuditReport { findings };
    assert!(!report.is_empty());
    assert_eq!(report.count(), 3);
}

#[test]
fn id_field_accepts_number_or_string() {
    let num: Advisory = serde_json::from_str(r#"{"id":42}"#).expect("number id");
    assert_eq!(num.id, serde_json::json!(42));
    let s: Advisory = serde_json::from_str(r#"{"id":"GHSA-xxxx"}"#).expect("string id");
    assert_eq!(s.id, serde_json::json!("GHSA-xxxx"));
}
