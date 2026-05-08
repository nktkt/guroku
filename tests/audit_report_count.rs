use std::collections::BTreeMap;

use guroku::audit::{Advisory, AuditReport};

fn adv() -> Advisory {
    Advisory {
        id: serde_json::json!(0),
        title: String::new(),
        severity: String::new(),
        url: String::new(),
        vulnerable_versions: String::new(),
        patched_versions: String::new(),
    }
}

#[test]
fn default_is_empty() {
    let r = AuditReport::default();
    assert!(r.is_empty());
}

#[test]
fn default_count_is_zero() {
    let r = AuditReport::default();
    assert_eq!(r.count(), 0);
}

#[test]
fn single_package_one_advisory() {
    let mut findings: BTreeMap<String, Vec<Advisory>> = BTreeMap::new();
    findings.insert("a".to_string(), vec![adv()]);
    let r = AuditReport { findings };
    assert!(!r.is_empty());
    assert_eq!(r.count(), 1);
}

#[test]
fn single_package_multiple_advisories() {
    let mut findings: BTreeMap<String, Vec<Advisory>> = BTreeMap::new();
    findings.insert("a".to_string(), vec![adv(), adv(), adv()]);
    let r = AuditReport { findings };
    assert_eq!(r.count(), 3);
}

#[test]
fn multiple_packages_advisories_summed() {
    let mut findings: BTreeMap<String, Vec<Advisory>> = BTreeMap::new();
    findings.insert("a".to_string(), vec![adv()]);
    findings.insert("b".to_string(), vec![adv(), adv()]);
    let r = AuditReport { findings };
    assert_eq!(r.count(), 3);
    assert_eq!(r.findings.len(), 2);
}

#[test]
fn empty_advisory_vec_counts_as_zero() {
    let mut findings: BTreeMap<String, Vec<Advisory>> = BTreeMap::new();
    findings.insert("a".to_string(), vec![]);
    let r = AuditReport { findings };
    assert_eq!(r.count(), 0);
    assert!(!r.is_empty());
}
