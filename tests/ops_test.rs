//! Write and schema operations on a working copy of the xiyouji fixture —
//! every transaction is exercised for both its success path and its refusal.

use laplace::model::{EntityRef, RelEntry};
use laplace::validate::Severity;
use laplace::{ops, schema_ops, validate, vault};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap().flatten() {
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// A scratch copy of the whole xiyouji fixture project (chapters + vault).
fn scratch() -> (tempfile::TempDir, PathBuf) {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/xiyouji");
    let tmp = tempfile::tempdir().unwrap();
    copy_dir(&src, tmp.path());
    let vault_dir = tmp.path().join("laplace");
    (tmp, vault_dir)
}

fn assert_clean(dir: &Path) {
    let v = vault::load(dir).unwrap();
    let r = validate::run(&v);
    let errs: Vec<_> = r
        .diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.render())
        .collect();
    assert!(errs.is_empty(), "{errs:#?}");
}

#[test]
fn add_creates_a_valid_entity_and_refuses_dangling() {
    let (_tmp, dir) = scratch();
    let v = vault::load(&dir).unwrap();
    let relations = {
        let mut m: BTreeMap<String, Vec<RelEntry>> = BTreeMap::new();
        m.insert(
            "结义".into(),
            vec![RelEntry::Bare("character:牛魔王".into())],
        );
        m
    };
    let out = ops::add(
        &v,
        ops::AddSpec {
            kind: "character".into(),
            name: "红孩儿".into(),
            body: "牛魔王与铁扇公主之子，号圣婴大王，善三昧真火。".into(),
            tags: vec!["妖王".into()],
            relations,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(out.message.contains("added character:default/红孩儿"));
    assert!(dir.join("character/红孩儿.md").is_file());
    assert_clean(&dir);

    // Dangling target → refused, nothing written.
    let v = vault::load(&dir).unwrap();
    let mut bad = BTreeMap::new();
    bad.insert(
        "结义".into(),
        vec![RelEntry::Bare("character:不存在".into())],
    );
    let err = ops::add(
        &v,
        ops::AddSpec {
            kind: "character".into(),
            name: "妖精甲".into(),
            body: "x".into(),
            relations: bad,
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("no such entity"), "{err}");
    assert!(!dir.join("character/妖精甲.md").exists());

    // Duplicate add → refused with a search hint.
    let err = ops::add(
        &v,
        ops::AddSpec {
            kind: "character".into(),
            name: "孙悟空".into(),
            body: "x".into(),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("already exists"));
}

#[test]
fn link_echoes_unlink_removes_and_symmetric_dedupes() {
    let (_tmp, dir) = scratch();
    let v = vault::load(&dir).unwrap();
    let out = ops::link(&v, "character:哪吒", "结义", "character:巨灵神", None).unwrap();
    assert!(out.message.contains("now 结义: ["), "{}", out.message);
    assert_clean(&dir);

    // Symmetric: declaring the reverse is a no-op, not a second edge.
    let v = vault::load(&dir).unwrap();
    let out = ops::link(&v, "character:巨灵神", "结义", "character:哪吒", None).unwrap();
    assert!(out.message.contains("no-op"), "{}", out.message);

    // Endpoint violation refused: 结义 is character↔character.
    let err = ops::link(&v, "character:哪吒", "结义", "artifact:如意金箍棒", None).unwrap_err();
    assert!(err.to_string().contains("bad-endpoint"), "{err}");

    let out = ops::unlink(&v, "character:哪吒", "结义", "character:巨灵神").unwrap();
    assert!(out.message.contains("unlinked"));
    assert_clean(&dir);
}

#[test]
fn remove_refuses_inbound_then_succeeds_after_unlink() {
    let (_tmp, dir) = scratch();
    let v = vault::load(&dir).unwrap();
    // 菩提祖师 has inbound (孙悟空 师从 him) → refused with the list.
    let err = ops::remove(&v, "character:菩提祖师").unwrap_err();
    assert!(
        err.to_string().contains("inbound refs would dangle"),
        "{err}"
    );
    assert!(err.to_string().contains("孙悟空"), "{err}");

    ops::unlink(&v, "character:孙悟空", "师从", "character:菩提祖师").unwrap();
    let _v = vault::load(&dir).unwrap();
    // Still has 出现于/涉及 inbound? 菩提祖师 is target of 涉及 from events.
    // Remove those first — count down to zero, then removal succeeds.
    loop {
        let v = vault::load(&dir).unwrap();
        match ops::remove(&v, "character:菩提祖师") {
            Ok(_) => break,
            Err(e) => {
                let msg = e.to_string();
                let line = msg
                    .lines()
                    .find(|l| l.contains("--") && l.contains("--> it"))
                    .map(|s| s.trim().to_string());
                let Some(line) = line else {
                    panic!("unparseable refusal: {msg}")
                };
                // "kind:name --rel--> it"
                let mut parts = line.split_whitespace();
                let src = parts.next().unwrap().to_string();
                let rel = parts
                    .next()
                    .unwrap()
                    .trim_matches(|c| c == '-' || c == '>')
                    .to_string();
                ops::unlink(&v, &src, &rel, "character:菩提祖师").unwrap();
            }
        }
    }
    assert!(!dir.join("character/菩提祖师.md").exists());
    assert_clean(&dir);
}

#[test]
fn rename_rewrites_inbound_and_reports_prose_mentions() {
    let (_tmp, dir) = scratch();
    let v = vault::load(&dir).unwrap();
    let out = ops::rename(&v, "character:牛魔王", "平天大圣", None).unwrap();
    assert!(out.message.contains("renamed"), "{}", out.message);
    assert!(dir.join("character/平天大圣.md").is_file());
    assert!(!dir.join("character/牛魔王.md").exists());
    // Inbound refs rewritten (结义 from 孙悟空 etc.), vault still clean.
    assert_clean(&dir);
    // Prose mentions reported but untouched.
    let json = out.json;
    assert!(json["rewritten_files"].as_u64().unwrap() >= 1, "{json}");
    if json["prose_mentions"].as_u64().unwrap() > 0 {
        assert!(out.message.contains("prose mentions"), "{}", out.message);
    }
}

#[test]
fn schema_ops_govern_the_constitution() {
    let (_tmp, dir) = scratch();
    let v = vault::load(&dir).unwrap();

    // add-relation without a reading direction → refused.
    let err =
        schema_ops::add_relation(&v, "克制", schema_ops::RelationSpec::default()).unwrap_err();
    assert!(err.to_string().contains("reading direction"), "{err}");

    schema_ops::add_relation(
        &v,
        "克制",
        schema_ops::RelationSpec {
            description: "A 克制 B —— A 是克星，B 被克制。".into(),
            symmetric: false,
            from: Some(vec!["artifact".into()]),
            to: Some(vec!["character".into()]),
            ..Default::default()
        },
    )
    .unwrap();
    assert_clean(&dir);

    // The new type is immediately usable.
    let v = vault::load(&dir).unwrap();
    ops::link(&v, "artifact:金刚琢", "克制", "character:孙悟空", None).unwrap();
    assert_clean(&dir);

    // rename-relation rewrites every usage.
    let v = vault::load(&dir).unwrap();
    let out = schema_ops::rename_relation(&v, "持有", "执掌").unwrap();
    assert!(out.message.contains("rewrote"), "{}", out.message);
    assert_clean(&dir);
    let v = vault::load(&dir).unwrap();
    assert!(v.schema.relations.contains_key("执掌"));
    assert!(!v.schema.relations.contains_key("持有"));
    let wukong = v
        .get(&EntityRef::parse("character:孙悟空").unwrap())
        .unwrap();
    assert!(wukong.fm.relations.contains_key("执掌"));

    // rename-kind moves the directory and rewrites refs.
    let v = vault::load(&dir).unwrap();
    schema_ops::rename_kind(&v, "artifact", "法宝").unwrap();
    assert!(dir.join("法宝/如意金箍棒.md").is_file());
    assert!(!dir.join("artifact").exists());
    assert_clean(&dir);

    // set flips a propagation and validates legality.
    let v = vault::load(&dir).unwrap();
    schema_ops::set(&v, "relations.执掌.propagation", "both").unwrap();
    let err = schema_ops::set(&v, "relations.结义.propagation", "to-source").unwrap_err();
    assert!(err.to_string().contains("errors"), "{err}");
    assert_clean(&dir);
}

#[test]
fn drift_reports_stale_and_uncovered_in_a_git_repo() {
    let (tmp, dir) = scratch();
    let root = tmp.path();
    let git = |args: &[&str]| {
        let ok = std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@t")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@t")
            .output()
            .unwrap();
        assert!(
            ok.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&ok.stderr)
        );
    };
    git(&["init", "-q"]);
    git(&["add", "-A"]);
    git(&["commit", "-qm", "init"]);

    // Touch a chapter (孙悟空 and 第一回 anchor it) and invent a new file.
    std::fs::write(root.join("chapters/ch01.md"), "重写第一回。\n").unwrap();
    std::fs::write(root.join("chapters/ch08.md"), "新的一回，无人认领。\n").unwrap();

    let v = vault::load(&dir).unwrap();
    let out = laplace::drift::run(&v, None).unwrap();
    assert_eq!(out["available"], true, "{out}");
    let stale: Vec<&str> = out["stale"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["ref"].as_str().unwrap())
        .collect();
    assert!(stale.contains(&"character:default/孙悟空"), "{stale:?}");
    assert!(stale.contains(&"chapter:default/第一回"), "{stale:?}");
    let uncovered: Vec<&str> = out["uncovered"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|p| p.as_str())
        .collect();
    assert!(uncovered.contains(&"chapters/ch08.md"), "{uncovered:?}");
    assert!(out["unanchored"]["count"].as_u64().unwrap() > 0);
}

#[test]
fn drift_degrades_explicitly_without_git() {
    let (_tmp, dir) = scratch();
    let v = vault::load(&dir).unwrap();
    let out = laplace::drift::run(&v, None).unwrap();
    assert_eq!(out["available"], false);
    assert!(out["notice"].as_str().unwrap().contains("git"));
}
