//! Smoke tests for `guroku::audit::print_report`.
//!
//! We can't easily capture stdout without unsafe stdio redirection, so
//! these tests just confirm the function runs to completion on a few
//! representative inputs without panicking.

use guroku::audit::{Advisory, AuditReport};
use std::collections::BTreeMap;

fn adv(title: &str) -> Advisory {
    Advisory {
        id: serde_json::json!("GHSA-x"),
        title: title.to_string(),
        severity: "moderate".to_string(),
        url: String::new(),
        vulnerable_versions: "<1".to_string(),
        patched_versions: ">=1".to_string(),
    }
}

#[test]
fn print_empty_report_does_not_panic() {
    guroku::audit::print_report(&AuditReport::default());
}

#[test]
fn print_single_finding_does_not_panic() {
    let mut findings: BTreeMap<String, Vec<Advisory>> = BTreeMap::new();
    findings.insert("left-pad".to_string(), vec![adv("oops")]);
    let report = AuditReport { findings };
    guroku::audit::print_report(&report);
}

#[test]
fn print_multiple_findings_does_not_panic() {
    let mut findings: BTreeMap<String, Vec<Advisory>> = BTreeMap::new();
    findings.insert("left-pad".to_string(), vec![adv("first"), adv("second")]);
    findings.insert("right-pad".to_string(), vec![adv("third")]);
    let report = AuditReport { findings };
    guroku::audit::print_report(&report);
}

#[test]
fn print_advisory_with_empty_optional_fields() {
    let advisory = Advisory {
        id: serde_json::json!("GHSA-y"),
        title: String::new(),
        severity: "high".to_string(),
        url: String::new(),
        vulnerable_versions: "<2".to_string(),
        patched_versions: String::new(),
    };
    let mut findings: BTreeMap<String, Vec<Advisory>> = BTreeMap::new();
    findings.insert("bare".to_string(), vec![advisory]);
    let report = AuditReport { findings };
    guroku::audit::print_report(&report);
}

#[test]
fn count_helper_returns_expected() {
    let mut findings: BTreeMap<String, Vec<Advisory>> = BTreeMap::new();
    findings.insert("left-pad".to_string(), vec![adv("first"), adv("second")]);
    findings.insert("right-pad".to_string(), vec![adv("third")]);
    let report = AuditReport { findings };
    assert_eq!(report.count(), 3);
    assert_eq!(report.findings.len(), 2);
}
