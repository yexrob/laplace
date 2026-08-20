//! Summary projection: tier behavior under budgets, and the skill's
//! delivery surfaces (embedded text, install, MCP instructions).

use laplace::{skill, summary, vault};
use std::path::PathBuf;

fn xiyouji() -> laplace::vault::Vault {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/xiyouji/laplace");
    vault::load(&dir).expect("vault loads")
}

#[test]
fn generous_budget_reaches_tier_3_with_names() {
    let v = xiyouji();
    let s = summary::render(&v, 4000);
    assert_eq!(s.tier, 3, "{}", s.text);
    assert!(
        s.text
            .starts_with("<laplace-map project=\"西游记·大闹天宫（前七回）\"")
    );
    assert!(s.text.contains("charter: "));
    assert!(s.text.contains("kinds: artifact(10)"), "{}", s.text);
    assert!(s.text.contains("relation-types: 出现于(99)"), "{}", s.text);
    assert!(s.text.contains("孙悟空"), "names present");
    assert!(s.text.contains("This map is authoritative"));
    assert!(s.text.trim_end().ends_with("</laplace-map>"));
    assert!(s.tokens <= 4000);
}

#[test]
fn tight_budget_truncates_lists_with_markers() {
    let v = xiyouji();
    let s = summary::render(&v, 500);
    assert!(s.tokens <= 500, "estimated {} > budget", s.tokens);
    if s.tier == 3 {
        assert!(s.text.contains("…+"), "truncation disclosed: {}", s.text);
    }
}

#[test]
fn floor_budget_still_emits_t0_contract() {
    let v = xiyouji();
    let s = summary::render(&v, 10);
    assert_eq!(s.tier, 0);
    // T0 always carries the header, charter, kinds, and the discipline lines —
    // the parts harnesses and the skill anchor on.
    assert!(s.text.contains("<laplace-map "));
    assert!(s.text.contains("charter: "));
    assert!(s.text.contains("kinds: "));
    assert!(s.text.contains("update the map in the same turn"));
    assert!(!s.text.contains("relation-types:"), "T1 dropped");
    assert!(!s.text.contains("孙悟空,"), "T3 dropped");
}

#[test]
fn skill_text_is_embedded_and_installable() {
    assert!(skill::SKILL_TEXT.starts_with("---\nname: entity-map"));
    assert!(skill::SKILL_TEXT.contains("Same-turn updates"));
    assert!(skill::SKILL_TEXT.contains("First use: initialize before consulting"));
    assert!(skill::SKILL_TEXT.contains("Scouts are read-only"));
    assert!(skill::SKILL_TEXT.contains("single writer"));
    assert!(skill::SKILL_TEXT.contains("laplace init"));
    assert!(skill::SKILL_TEXT.contains("Populate in two passes"));
    assert!(skill::SKILL_TEXT.contains("not proof of total repository coverage"));
    assert!(skill::SKILL_TEXT.contains("do not commit merely because"));
    assert!(skill::SKILL_TEXT.contains("repository-wide `source` glob"));
    assert!(skill::SKILL_TEXT.contains("If a matching vault already exists, skip bootstrap"));
    assert!(skill::SKILL_TEXT.contains("When no matching vault exists, initialize only"));
    assert!(skill::SKILL_TEXT.contains("pass `--vault"));
    assert!(skill::SKILL_TEXT.contains("temporary\nfile first"));
    assert!(skill::SKILL_TEXT.contains("drift` cannot audit a project-root vault"));
    assert!(skill::MCP_INSTRUCTIONS.contains("laplace_schema"));

    let tmp = tempfile::tempdir().unwrap();
    let paths = skill::install(tmp.path(), Some(&tmp.path().join("skills"))).unwrap();
    assert_eq!(paths.len(), 1);
    assert!(paths[0].ends_with("entity-map/SKILL.md"));
    assert_eq!(
        std::fs::read_to_string(&paths[0]).unwrap(),
        skill::SKILL_TEXT
    );

    // Project-level detection: a .bingo dir in cwd gets offered.
    std::fs::create_dir_all(tmp.path().join(".bingo")).unwrap();
    let detected = skill::detect_targets(tmp.path());
    assert!(detected.iter().any(|d| d.ends_with(".bingo/skills")));
}
