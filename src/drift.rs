//! Drift (SPEC §6): the session-start freshness audit. Git-driven; every kind
//! of blindness is disclosed, so silence is never mistaken for cleanliness.

use crate::validate::walk_project_files;
use crate::vault::Vault;
use anyhow::Result;
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn git(root: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

fn globset_of(globs: &[String]) -> Option<GlobSet> {
    let mut b = GlobSetBuilder::new();
    for g in globs {
        b.add(Glob::new(g).ok()?);
    }
    b.build().ok()
}

pub fn run(vault: &Vault, since: Option<&str>) -> Result<Value> {
    let root = &vault.project_root;
    let Some(repo_root) = git(root, &["rev-parse", "--show-toplevel"]) else {
        return Ok(json!({
            "available": false,
            "notice": format!(
                "{} is not inside a git repository — drift needs git history; freshness falls back to discipline and review",
                root.display()
            ),
        }));
    };
    let repo_root = std::path::PathBuf::from(repo_root.trim());
    // Project root relative to the repo root, for translating git paths.
    let proj_prefix = root
        .canonicalize()
        .ok()
        .and_then(|r| r.strip_prefix(&repo_root).map(|p| p.to_path_buf()).ok())
        .unwrap_or_default();
    let to_project = |repo_path: &str| -> Option<String> {
        let p = Path::new(repo_path);
        let rel = p.strip_prefix(&proj_prefix).ok()?;
        Some(rel.to_string_lossy().replace('\\', "/"))
    };
    // The vault itself, project-relative: map maintenance is not territory.
    let vault_prefix = vault
        .dir
        .canonicalize()
        .ok()
        .and_then(|v| {
            root.canonicalize().ok().and_then(|r| {
                v.strip_prefix(r)
                    .map(|p| p.to_string_lossy().into_owned())
                    .ok()
            })
        })
        .unwrap_or_else(|| "laplace".into());

    // Base: the last commit touching the vault (or --since).
    let vault_repo_rel = proj_prefix.join(&vault_prefix);
    let base = match since {
        Some(rev) => Some(rev.to_string()),
        None => git(
            &repo_root,
            &[
                "log",
                "-n1",
                "--format=%H",
                "--",
                &vault_repo_rel.to_string_lossy(),
            ],
        )
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty()),
    };
    let mut notice = None;
    let committed_changes = match &base {
        Some(b) => {
            git(&repo_root, &["diff", "--name-only", &format!("{b}..HEAD")]).unwrap_or_default()
        }
        None => {
            notice = Some(
                "the vault has no commits yet — only working-tree changes are considered"
                    .to_string(),
            );
            String::new()
        }
    };
    let dirty = git(&repo_root, &["status", "--porcelain"]).unwrap_or_default();
    let ignore_set = globset_of(&vault.schema.ignore);

    let mut changed: BTreeSet<String> = BTreeSet::new();
    for line in committed_changes.lines() {
        if let Some(p) = to_project(line.trim()) {
            changed.insert(p);
        }
    }
    for line in dirty.lines() {
        // porcelain: "XY path" (or "XY old -> new" for renames)
        let path = line.get(3..).unwrap_or("").trim();
        let path = path.split(" -> ").last().unwrap_or(path);
        if let Some(p) = to_project(path) {
            changed.insert(p);
        }
    }
    changed.retain(|p| {
        !p.is_empty()
            && !Path::new(p).starts_with(&vault_prefix)
            && !ignore_set.as_ref().is_some_and(|s| s.is_match(p))
    });

    // Tracked set for the dead-anchor (drift flavor) signal.
    let tracked: BTreeSet<String> = git(&repo_root, &["ls-files"])
        .unwrap_or_default()
        .lines()
        .filter_map(|l| to_project(l.trim()))
        .collect();
    let disk_files = walk_project_files(root);

    let mut stale = Vec::new();
    let mut dead_anchors = Vec::new();
    let mut unanchored = 0usize;
    let mut covering: Vec<GlobSet> = Vec::new();
    for e in &vault.entities {
        if e.fm.source.is_empty() {
            unanchored += 1;
            continue;
        }
        let Some(set) = globset_of(&e.fm.source) else {
            continue; // bad-glob is validate's finding
        };
        let hits: Vec<&String> = changed.iter().filter(|p| set.is_match(p)).collect();
        if !hits.is_empty() {
            stale.push(json!({
                "ref": e.eref.to_string(),
                "paths": hits,
            }));
        }
        let disk_hits: Vec<&String> = disk_files.iter().filter(|p| set.is_match(p)).collect();
        if !disk_hits.is_empty() && !disk_hits.iter().any(|p| tracked.contains(*p)) {
            dead_anchors.push(json!({
                "ref": e.eref.to_string(),
                "reason": "anchors resolve only to git-untracked/ignored paths — drift can never watch them",
            }));
        }
        covering.push(set);
    }
    let uncovered: Vec<&String> = changed
        .iter()
        .filter(|p| !covering.iter().any(|s| s.is_match(p)))
        .collect();

    let total = vault.entities.len().max(1);
    Ok(json!({
        "available": true,
        "base": base,
        "notice": notice,
        "changed_paths": changed.len(),
        "stale": stale,
        "uncovered": uncovered,
        "dead_anchors": dead_anchors,
        "unanchored": { "count": unanchored, "ratio": format!("{:.0}%", 100.0 * unanchored as f64 / total as f64) },
    }))
}

pub fn render_text(v: &Value) -> String {
    if !v["available"].as_bool().unwrap_or(false) {
        return v["notice"]
            .as_str()
            .unwrap_or("drift unavailable")
            .to_string();
    }
    let mut out = String::new();
    if let Some(n) = v["notice"].as_str() {
        out.push_str(&format!("note: {n}\n"));
    }
    let stale = v["stale"].as_array().unwrap();
    if stale.is_empty() {
        out.push_str("stale: none — no anchored entity's territory changed\n");
    } else {
        out.push_str(&format!(
            "stale ({} — the map may misdescribe these):\n",
            stale.len()
        ));
        for s in stale {
            let paths: Vec<&str> = s["paths"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|p| p.as_str())
                .collect();
            out.push_str(&format!(
                "  {}  ← {}\n",
                s["ref"].as_str().unwrap(),
                paths.join(", ")
            ));
        }
    }
    let uncovered = v["uncovered"].as_array().unwrap();
    if uncovered.is_empty() {
        out.push_str("uncovered: none\n");
    } else {
        out.push_str(&format!(
            "uncovered ({} changed paths no entity claims — unmapped territory):\n",
            uncovered.len()
        ));
        for p in uncovered {
            out.push_str(&format!("  {}\n", p.as_str().unwrap()));
        }
    }
    for d in v["dead_anchors"].as_array().unwrap() {
        out.push_str(&format!(
            "dead-anchor: {} — {}\n",
            d["ref"].as_str().unwrap(),
            d["reason"].as_str().unwrap()
        ));
    }
    let ua = &v["unanchored"];
    out.push_str(&format!(
        "unanchored: {} entities ({}) have no source anchors — known blindness, not cleanliness\n",
        ua["count"],
        ua["ratio"].as_str().unwrap()
    ));
    out
}
