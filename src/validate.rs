//! The three validation layers plus fixture-earned checks (SPEC §3).
//! Validate is the reconciler for whatever bypassed the write path, and the CI gate.

use crate::model::{EntityRef, Propagation, levenshtein_capped};
use crate::vault::Vault;
use globset::{Glob, GlobSetBuilder};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Serialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

impl Diagnostic {
    pub fn err(code: &'static str, file: Option<PathBuf>, message: String) -> Self {
        Self {
            severity: Severity::Error,
            code,
            file,
            line: None,
            entity: None,
            path: None,
            message,
            suggestion: None,
        }
    }

    pub fn render(&self) -> String {
        let mut loc = String::new();
        if let Some(f) = &self.file {
            loc.push_str(&f.display().to_string());
            if let Some(l) = self.line {
                loc.push_str(&format!(":{l}"));
            }
        }
        if let Some(e) = &self.entity {
            loc.push_str(&format!(" ({e})"));
        }
        if let Some(p) = &self.path {
            loc.push_str(&format!(" {p}"));
        }
        let sev = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        let mut out = if loc.is_empty() {
            format!("{sev}[{}]: {}", self.code, self.message)
        } else {
            format!("{loc}\n  {sev}[{}]: {}", self.code, self.message)
        };
        if let Some(s) = &self.suggestion {
            out.push_str(&format!("\n  → {s}"));
        }
        out
    }
}

pub struct Report {
    pub diags: Vec<Diagnostic>,
}

impl Report {
    pub fn errors(&self) -> usize {
        self.diags
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count()
    }
    pub fn warnings(&self) -> usize {
        self.diags.len() - self.errors()
    }
}

pub fn run(vault: &Vault) -> Report {
    let mut diags: Vec<Diagnostic> = Vec::new();
    diags.extend(vault.load_diags.iter().map(clone_diag));

    schema_checks(vault, &mut diags);

    let refs: BTreeSet<EntityRef> = vault.entities.iter().map(|e| e.eref.clone()).collect();

    // Edge collection for graph-level checks; only resolvable edges enter.
    let mut edges: Vec<(EntityRef, String, EntityRef)> = Vec::new();
    let mut seen_edge: HashSet<(EntityRef, String, EntityRef)> = HashSet::new();
    let mut seen_sym: HashMap<(String, EntityRef, EntityRef), EntityRef> = HashMap::new();

    for e in &vault.entities {
        if !vault.schema.kinds.contains_key(&e.eref.kind) {
            diags.push(Diagnostic {
                severity: Severity::Error,
                code: "unknown-kind",
                file: Some(e.file.clone()),
                line: None,
                entity: Some(e.eref.to_string()),
                path: None,
                message: format!("kind `{}` is not declared in schema.yaml", e.eref.kind),
                suggestion: closest(&e.eref.kind, vault.schema.kinds.keys())
                    .map(|k| format!("did you mean kind `{k}`? or declare it under kinds:")),
            });
        }
        for (rel, entries) in &e.fm.relations {
            let Some(decl) = vault.schema.relations.get(rel) else {
                diags.push(Diagnostic {
                    severity: Severity::Error,
                    code: "undeclared-relation",
                    file: Some(e.file.clone()),
                    line: e.line_of(rel),
                    entity: Some(e.eref.to_string()),
                    path: Some(format!("relations.{rel}")),
                    message: format!("relation type `{rel}` is not declared in schema.yaml"),
                    suggestion: closest(rel, vault.schema.relations.keys())
                        .map(|r| format!("did you mean `{r}`? or declare it under relations:")),
                });
                continue;
            };
            for entry in entries {
                let raw_target = entry.target();
                let target = match EntityRef::parse(raw_target) {
                    Ok(t) => t,
                    Err(msg) => {
                        diags.push(Diagnostic {
                            severity: Severity::Error,
                            code: "bad-ref",
                            file: Some(e.file.clone()),
                            line: e.line_of(raw_target),
                            entity: Some(e.eref.to_string()),
                            path: Some(format!("relations.{rel}")),
                            message: msg,
                            suggestion: None,
                        });
                        continue;
                    }
                };
                if !refs.contains(&target) {
                    diags.push(Diagnostic {
                        severity: Severity::Error,
                        code: "dangling-ref",
                        file: Some(e.file.clone()),
                        line: e.line_of(raw_target),
                        entity: Some(e.eref.to_string()),
                        path: Some(format!("relations.{rel} → {target}")),
                        message: "no such entity".into(),
                        suggestion: did_you_mean(&target, &refs)
                            .map(|r| format!("did you mean {r}?")),
                    });
                    continue;
                }
                if let Some(from) = &decl.from
                    && !from.contains(&e.eref.kind)
                {
                    diags.push(bad_endpoint(
                        e,
                        rel,
                        &format!(
                            "`{rel}` edges start from {from:?}, not from kind `{}`",
                            e.eref.kind
                        ),
                    ));
                }
                if let Some(to) = &decl.to
                    && !to.contains(&target.kind)
                {
                    diags.push(bad_endpoint(
                        e,
                        rel,
                        &format!(
                            "`{rel}` edges point at {to:?}, not at kind `{}` ({target})",
                            target.kind
                        ),
                    ));
                }
                let key = (e.eref.clone(), rel.clone(), target.clone());
                if !seen_edge.insert(key.clone()) {
                    diags.push(Diagnostic {
                        severity: Severity::Warning,
                        code: "duplicate-edge",
                        file: Some(e.file.clone()),
                        line: e.line_of(raw_target),
                        entity: Some(e.eref.to_string()),
                        path: Some(format!("relations.{rel} → {target}")),
                        message: "same edge stated twice; deduplicated".into(),
                        suggestion: None,
                    });
                    continue;
                }
                if decl.symmetric {
                    let (a, b) = ordered(&e.eref, &target);
                    if let Some(first) = seen_sym.insert((rel.clone(), a, b), e.eref.clone())
                        && first != e.eref
                    {
                        diags.push(Diagnostic {
                                severity: Severity::Warning,
                                code: "symmetric-declared-twice",
                                file: Some(e.file.clone()),
                                line: e.line_of(raw_target),
                                entity: Some(e.eref.to_string()),
                                path: Some(format!("relations.{rel} → {target}")),
                                message: format!(
                                    "`{rel}` is symmetric and already declared on {first}; one side suffices"
                                ),
                                suggestion: None,
                            });
                        continue;
                    }
                }
                edges.push((e.eref.clone(), rel.clone(), target));
            }
        }
    }

    orphans(vault, &edges, &mut diags);
    acyclic_checks(vault, &edges, &mut diags);
    anchor_checks(vault, &mut diags);

    order(&mut diags);
    Report { diags }
}

fn schema_checks(vault: &Vault, diags: &mut Vec<Diagnostic>) {
    for (name, decl) in &vault.schema.relations {
        if decl
            .description
            .as_deref()
            .is_none_or(|d| d.trim().is_empty())
        {
            diags.push(Diagnostic {
                severity: Severity::Error,
                code: "missing-relation-description",
                file: Some(PathBuf::from("schema.yaml")),
                line: None,
                entity: None,
                path: Some(format!("relations.{name}")),
                message: "every relation must state its reading direction (\"A rel B means…\") — direction confusion corrupts silently".into(),
                suggestion: None,
            });
        }
        if decl.symmetric && !matches!(decl.propagation, Propagation::Both | Propagation::None) {
            diags.push(Diagnostic {
                severity: Severity::Error,
                code: "bad-propagation",
                file: Some(PathBuf::from("schema.yaml")),
                line: None,
                entity: None,
                path: Some(format!("relations.{name}")),
                message: format!(
                    "symmetric relations have no direction: propagation must be `both` or `none`, not `{}`",
                    decl.propagation.as_str()
                ),
                suggestion: None,
            });
        }
    }
}

fn orphans(vault: &Vault, edges: &[(EntityRef, String, EntityRef)], diags: &mut Vec<Diagnostic>) {
    let mut connected: HashSet<&EntityRef> = HashSet::new();
    for (s, _, t) in edges {
        connected.insert(s);
        connected.insert(t);
    }
    for e in &vault.entities {
        if !connected.contains(&e.eref) {
            diags.push(Diagnostic {
                severity: Severity::Warning,
                code: "orphan",
                file: Some(e.file.clone()),
                line: None,
                entity: Some(e.eref.to_string()),
                path: None,
                message: "no edges in either direction — dead weight until something relates to it"
                    .into(),
                suggestion: None,
            });
        }
    }
}

fn acyclic_checks(
    vault: &Vault,
    edges: &[(EntityRef, String, EntityRef)],
    diags: &mut Vec<Diagnostic>,
) {
    for (rel, decl) in &vault.schema.relations {
        if !decl.acyclic {
            continue;
        }
        let mut adj: BTreeMap<&EntityRef, Vec<&EntityRef>> = BTreeMap::new();
        for (s, r, t) in edges {
            if r == rel {
                adj.entry(s).or_default().push(t);
            }
        }
        // Iterative DFS three-color cycle detection.
        let mut state: HashMap<&EntityRef, u8> = HashMap::new();
        for &start in adj.keys() {
            if state.get(start).copied().unwrap_or(0) != 0 {
                continue;
            }
            let mut stack = vec![(start, 0usize)];
            let mut trail: Vec<&EntityRef> = Vec::new();
            while let Some((node, idx)) = stack.pop() {
                if idx == 0 {
                    state.insert(node, 1);
                    trail.push(node);
                }
                let next = adj.get(node).and_then(|v| v.get(idx));
                match next {
                    Some(&n) => {
                        stack.push((node, idx + 1));
                        match state.get(n).copied().unwrap_or(0) {
                            0 => stack.push((n, 0)),
                            1 => {
                                let pos = trail.iter().position(|x| *x == n).unwrap_or(0);
                                let cycle: Vec<String> = trail[pos..]
                                    .iter()
                                    .map(|r| r.to_string())
                                    .chain([n.to_string()])
                                    .collect();
                                diags.push(Diagnostic {
                                    severity: Severity::Error,
                                    code: "cycle",
                                    file: None,
                                    line: None,
                                    entity: Some(n.to_string()),
                                    path: Some(format!("relations.{rel}")),
                                    message: format!(
                                        "`{rel}` is declared acyclic but cycles: {}",
                                        cycle.join(" → ")
                                    ),
                                    suggestion: None,
                                });
                                stack.clear();
                                trail.clear();
                            }
                            _ => {}
                        }
                    }
                    None => {
                        state.insert(node, 2);
                        trail.pop();
                    }
                }
            }
        }
    }
}

/// dead-anchor (SPEC §3): a source glob matching nothing on disk. Walks the
/// project root once, honoring .gitignore, and tests every anchor against it.
fn anchor_checks(vault: &Vault, diags: &mut Vec<Diagnostic>) {
    let anchored: Vec<(&crate::model::Entity, &String)> = vault
        .entities
        .iter()
        .flat_map(|e| e.fm.source.iter().map(move |g| (e, g)))
        .collect();
    if anchored.is_empty() {
        return;
    }
    let files = walk_project_files(&vault.project_root);
    for (e, glob) in anchored {
        let Ok(g) = Glob::new(glob) else {
            diags.push(Diagnostic {
                severity: Severity::Error,
                code: "bad-glob",
                file: Some(e.file.clone()),
                line: e.line_of(glob),
                entity: Some(e.eref.to_string()),
                path: Some("source".into()),
                message: format!("`{glob}` is not a valid glob"),
                suggestion: None,
            });
            continue;
        };
        let mut b = GlobSetBuilder::new();
        b.add(g);
        let set = b.build().expect("single-glob set");
        if !files.iter().any(|f| set.is_match(f)) {
            diags.push(Diagnostic {
                severity: Severity::Warning,
                code: "dead-anchor",
                file: Some(e.file.clone()),
                line: e.line_of(glob),
                entity: Some(e.eref.to_string()),
                path: Some("source".into()),
                message: format!(
                    "`{glob}` matches nothing under {} — a renamed file degrades silently to \"never stale\"",
                    vault.project_root.display()
                ),
                suggestion: None,
            });
        }
    }
}

/// Walk the project root once, honoring .gitignore and skipping hidden files,
/// returning root-relative slash-separated paths. Shared by anchor checks and drift.
pub fn walk_project_files(root: &std::path::Path) -> Vec<String> {
    let mut files: Vec<String> = Vec::new();
    if root.is_dir() {
        for entry in ignore::WalkBuilder::new(root)
            .hidden(true)
            .build()
            .flatten()
        {
            if entry.file_type().is_some_and(|t| t.is_file())
                && let Ok(rel) = entry.path().strip_prefix(root)
            {
                files.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    files
}

fn bad_endpoint(e: &crate::model::Entity, rel: &str, msg: &str) -> Diagnostic {
    Diagnostic {
        severity: Severity::Error,
        code: "bad-endpoint",
        file: Some(e.file.clone()),
        line: e.line_of(rel),
        entity: Some(e.eref.to_string()),
        path: Some(format!("relations.{rel}")),
        message: msg.to_string(),
        suggestion: None,
    }
}

/// did-you-mean (SPEC §3): same kind at distance ≤ 2 first, then the same name
/// under another kind/namespace, then cross-kind distance ≤ 2.
pub fn did_you_mean(target: &EntityRef, refs: &BTreeSet<EntityRef>) -> Option<String> {
    let mut best: Option<(usize, &EntityRef)> = None;
    for r in refs {
        if r.kind == target.kind {
            let d = levenshtein_capped(&r.name, &target.name, 2);
            if d <= 2 && best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, r));
            }
        }
    }
    if let Some((_, r)) = best {
        return Some(r.to_string());
    }
    if let Some(r) = refs.iter().find(|r| r.name == target.name) {
        return Some(r.to_string());
    }
    refs.iter()
        .filter(|r| levenshtein_capped(&r.name, &target.name, 2) <= 2)
        .map(|r| r.to_string())
        .next()
}

fn closest<'a>(input: &str, candidates: impl Iterator<Item = &'a String>) -> Option<String> {
    candidates
        .map(|c| (levenshtein_capped(c, input, 2), c))
        .filter(|(d, _)| *d <= 2)
        .min_by_key(|(d, _)| *d)
        .map(|(_, c)| c.clone())
}

fn ordered(a: &EntityRef, b: &EntityRef) -> (EntityRef, EntityRef) {
    if a <= b {
        (a.clone(), b.clone())
    } else {
        (b.clone(), a.clone())
    }
}

fn order(diags: &mut [Diagnostic]) {
    diags.sort_by(|a, b| {
        (matches!(a.severity, Severity::Warning), &a.file, a.line).cmp(&(
            matches!(b.severity, Severity::Warning),
            &b.file,
            b.line,
        ))
    });
}

fn clone_diag(d: &Diagnostic) -> Diagnostic {
    Diagnostic {
        severity: d.severity,
        code: d.code,
        file: d.file.clone(),
        line: d.line,
        entity: d.entity.clone(),
        path: d.path.clone(),
        message: d.message.clone(),
        suggestion: d.suggestion.clone(),
    }
}
