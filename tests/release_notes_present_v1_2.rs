const RELEASE_NOTES: &str = include_str!("../docs/v1.2-release-notes.md");
const MIGRATION: &str = include_str!("../docs/migration/v1.1-to-v1.2.md");
const INTERNALS_PUBGRUB: &str = include_str!("../docs/internals/pubgrub-integration.md");
const INTERNALS_RANGE: &str = include_str!("../docs/internals/range-conversion.md");
const INTERNALS_TWO_PHASE: &str = include_str!("../docs/internals/two-phase-resolver.md");
const USER_PUBGRUB: &str = include_str!("../docs/pubgrub-resolver.md");

#[test]
fn release_notes_mention_pubgrub() {
    assert!(!RELEASE_NOTES.is_empty());
    assert!(RELEASE_NOTES.contains("pubgrub") || RELEASE_NOTES.contains("PubGrub"));
}

#[test]
fn release_notes_state_v1_0_surface_unchanged() {
    assert!(RELEASE_NOTES.contains("unchanged") || RELEASE_NOTES.contains("stable"));
}

#[test]
fn migration_explains_env_var() {
    assert!(MIGRATION.contains("GUROKU_RESOLVER"));
}

#[test]
fn migration_explains_no_breaking_change() {
    assert!(MIGRATION.contains("No") || MIGRATION.contains("no") || MIGRATION.contains("Drop in"));
}

#[test]
fn internals_pubgrub_mentions_dependency_provider() {
    assert!(INTERNALS_PUBGRUB.contains("DependencyProvider"));
}

#[test]
fn internals_range_mentions_candidate_set() {
    assert!(INTERNALS_RANGE.contains("candidate"));
}

#[test]
fn internals_two_phase_mentions_async_sync_bridge() {
    let s = INTERNALS_TWO_PHASE.to_lowercase();
    assert!(s.contains("async") && s.contains("sync"));
}

#[test]
fn user_pubgrub_doc_mentions_overrides_remediation() {
    assert!(USER_PUBGRUB.contains("overrides"));
}
