//! Validation against the two real fixture vaults, plus negative tests over
//! deliberately broken vaults — a validator that has never failed proves nothing.

use laplace::validate::{self, Severity};
use laplace::vault;
use std::path::PathBuf;

fn load(rel: &str) -> laplace::vault::Vault {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    vault::load(&dir).expect("vault loads")
}

fn codes(report: &validate::Report, severity: Severity) -> Vec<&'static str> {
    report
        .diags
        .iter()
        .filter(|d| d.severity == severity)
        .map(|d| d.code)
        .collect()
}

#[test]
fn xiyouji_is_clean() {
    let v = load("fixtures/xiyouji/laplace");
    let r = validate::run(&v);
    assert_eq!(r.errors(), 0, "{:#?}", codes(&r, Severity::Error));
    assert_eq!(r.warnings(), 0, "{:#?}", codes(&r, Severity::Warning));
    assert_eq!(v.entities.len(), 60);
}

#[test]
fn bingo_is_clean_with_dead_anchors() {
    // The bingo vault is a snapshot: anchors point at the real repo, which is
    // not inside the fixture — every anchor is dead here, and that is exactly
    // what the dead-anchor diagnostic must say.
    let v = load("fixtures/bingo/laplace");
    let r = validate::run(&v);
    assert_eq!(r.errors(), 0, "{:#?}", codes(&r, Severity::Error));
    let warns = codes(&r, Severity::Warning);
    assert!(warns.iter().all(|c| *c == "dead-anchor"), "{warns:#?}");
    assert!(r.warnings() > 100);
}

#[test]
fn dangling_undeclared_unknown() {
    let r = validate::run(&load("tests/broken/refs/laplace"));
    let errs = codes(&r, Severity::Error);
    assert!(errs.contains(&"dangling-ref"), "{errs:#?}");
    assert!(errs.contains(&"undeclared-relation"));
    assert!(errs.contains(&"unknown-kind"));
    let dangling = r
        .diags
        .iter()
        .find(|d| d.code == "dangling-ref")
        .expect("dangling diagnostic");
    assert_eq!(
        dangling.suggestion.as_deref(),
        Some("did you mean a:default/beta?")
    );
    assert!(
        dangling.line.is_some(),
        "dangling ref carries a line number"
    );
}

#[test]
fn endpoints_identity_duplicates_symmetric() {
    let r = validate::run(&load("tests/broken/endpoints/laplace"));
    let errs = codes(&r, Severity::Error);
    let warns = codes(&r, Severity::Warning);
    // b:bx target violates to:[a]; also ax owns b:bx violates nothing from-side
    // (ax is kind a), so exactly the to-violation plus identity-in-frontmatter.
    assert!(errs.contains(&"bad-endpoint"), "{errs:#?}");
    assert!(errs.contains(&"identity-in-frontmatter"));
    assert!(warns.contains(&"duplicate-edge"), "{warns:#?}");
    assert!(warns.contains(&"symmetric-declared-twice"));
}

#[test]
fn acyclic_cycle_and_orphan() {
    let r = validate::run(&load("tests/broken/cycle/laplace"));
    assert!(codes(&r, Severity::Error).contains(&"cycle"));
    assert!(codes(&r, Severity::Warning).contains(&"orphan"));
}

#[test]
fn schema_level_errors_and_bad_frontmatter() {
    let r = validate::run(&load("tests/broken/schema/laplace"));
    let errs = codes(&r, Severity::Error);
    assert!(errs.contains(&"missing-relation-description"), "{errs:#?}");
    assert!(errs.contains(&"bad-propagation"));
    assert!(errs.contains(&"bad-frontmatter"));
}

#[test]
fn queries_refuse_broken_vaults_is_enforced_in_cli() {
    // The library contract behind it: a broken vault still loads (total load),
    // and the report carries the errors the CLI gates on.
    let v = load("tests/broken/refs/laplace");
    assert!(validate::run(&v).errors() > 0);
    assert!(!v.entities.is_empty());
}
