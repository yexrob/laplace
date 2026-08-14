//! The write operations (SPEC §2). Every op follows one transactional shape:
//! precondition checks against the live graph → apply to an in-memory clone →
//! whole-vault re-validation (abort on any NEW error) → atomic persist.
//! Frontmatter and schema.yaml are machine-owned; bodies are never touched
//! except by an explicit body edit.

use crate::model::{Entity, EntityRef, FrontMatter, RelEntry, first_sentence, nfc};
use crate::validate::{self, Severity};
use crate::vault::{self, Vault};
use anyhow::{Result, anyhow, bail};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

#[derive(Debug)]
pub struct Outcome {
    pub message: String,
    pub json: Value,
}

#[derive(Debug, Clone)]
pub enum Change {
    Write { rel: PathBuf, content: String },
    Delete { rel: PathBuf },
}

/// Elided display form: default namespace omitted — the house style for refs
/// written into files (query output stays canonical).
pub fn elide(r: &EntityRef) -> String {
    if r.ns == "default" {
        format!("{}:{}", r.kind, r.name)
    } else {
        format!("{}:{}/{}", r.kind, r.ns, r.name)
    }
}

pub fn entity_path(r: &EntityRef) -> PathBuf {
    if r.ns == "default" {
        PathBuf::from(&r.kind).join(format!("{}.md", r.name))
    } else {
        PathBuf::from(&r.kind)
            .join(&r.ns)
            .join(format!("{}.md", r.name))
    }
}

/// Render an entity file: canonical frontmatter order, body untouched.
pub fn render_entity(fm: &FrontMatter, body: &str) -> String {
    let mut map = serde_norway::Mapping::new();
    let yv = |v: &Value| -> serde_norway::Value { serde_norway::to_value(v).unwrap() };
    if let Some(t) = &fm.title {
        map.insert("title".into(), yv(&json!(t)));
    }
    if !fm.tags.is_empty() {
        map.insert("tags".into(), yv(&json!(fm.tags)));
    }
    if let Some(l) = &fm.lifecycle {
        map.insert("lifecycle".into(), yv(&json!(l)));
    }
    if !fm.relations.is_empty() {
        map.insert(
            "relations".into(),
            serde_norway::to_value(&fm.relations).unwrap(),
        );
    }
    if !fm.source.is_empty() {
        map.insert("source".into(), yv(&json!(fm.source)));
    }
    for (k, v) in &fm.extra {
        map.insert(serde_norway::Value::String(k.clone()), v.clone());
    }
    let body = body.trim_end();
    if map.is_empty() {
        if body.is_empty() {
            String::new()
        } else {
            format!("{body}\n")
        }
    } else {
        let yaml = serde_norway::to_string(&map).expect("frontmatter serializes");
        format!("---\n{yaml}---\n{body}\n")
    }
}

/// Apply changes to a clone of the vault, re-validate, and abort on any error
/// that was not already present — repair of an already-broken vault stays legal.
fn simulate(current: &Vault, changes: &[Change]) -> Result<()> {
    let baseline: BTreeSet<String> = validate::run(current)
        .diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.render())
        .collect();
    let mut entities: Vec<Entity> = current.entities.clone();
    for c in changes {
        match c {
            Change::Delete { rel } => entities.retain(|e| &e.file != rel),
            Change::Write { rel, content } => {
                entities.retain(|e| &e.file != rel);
                let (parsed, diags) = vault::parse_entity(rel.clone(), content.clone());
                if let Some(d) = diags.iter().find(|d| d.severity == Severity::Error) {
                    bail!("the write would not parse back: {}", d.render());
                }
                if let Some(e) = parsed {
                    entities.push(e);
                }
            }
        }
    }
    let sim = Vault {
        dir: current.dir.clone(),
        project_root: current.project_root.clone(),
        schema: current.schema.clone(),
        entities,
        load_diags: Vec::new(),
    };
    let new_errors: Vec<String> = validate::run(&sim)
        .diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.render())
        .filter(|r| !baseline.contains(r))
        .collect();
    if new_errors.is_empty() {
        Ok(())
    } else {
        bail!(
            "refused — the operation would introduce errors:\n{}",
            new_errors.join("\n")
        )
    }
}

/// Atomic-ish persist: write every temp first, then rename all; deletes last.
/// Multi-file transactions rely on git as the rollback of last resort (SPEC §2).
fn persist(vault_dir: &std::path::Path, changes: &[Change]) -> Result<()> {
    let mut staged: Vec<(PathBuf, PathBuf)> = Vec::new();
    for c in changes {
        if let Change::Write { rel, content } = c {
            let target = vault_dir.join(rel);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let tmp = target.with_file_name(format!(
                ".laplace-tmp-{}",
                target.file_name().unwrap().to_string_lossy()
            ));
            std::fs::write(&tmp, content)?;
            staged.push((tmp, target));
        }
    }
    for (tmp, target) in staged {
        std::fs::rename(tmp, target)?;
    }
    for c in changes {
        if let Change::Delete { rel } = c {
            std::fs::remove_file(vault_dir.join(rel))?;
        }
    }
    Ok(())
}

fn commit(vault: &Vault, changes: Vec<Change>) -> Result<()> {
    simulate(vault, &changes)?;
    persist(&vault.dir, &changes)
}

/// Resolve with a did-you-mean in the error message.
fn resolve(vault: &Vault, input: &str) -> Result<EntityRef> {
    let r = EntityRef::parse(input).map_err(|e| anyhow!(e))?;
    if vault.get(&r).is_some() {
        return Ok(r);
    }
    let refs: BTreeSet<EntityRef> = vault.entities.iter().map(|e| e.eref.clone()).collect();
    match validate::did_you_mean(&r, &refs) {
        Some(s) => bail!("no such entity: {r} — did you mean {s}?"),
        None => bail!(
            "no such entity: {r} (try `laplace query search {}`)",
            r.name
        ),
    }
}

fn check_relation<'a>(vault: &'a Vault, rel: &str) -> Result<&'a crate::model::RelationDecl> {
    vault.schema.relations.get(rel).ok_or_else(|| {
        let close = vault
            .schema
            .relations
            .keys()
            .find(|k| crate::model::levenshtein_capped(k, rel, 2) <= 2)
            .map(|k| format!(" — did you mean `{k}`?"))
            .unwrap_or_else(|| " (declare it first: `laplace schema add-relation`)".into());
        anyhow!("relation type `{rel}` is not declared{close}")
    })
}

fn check_endpoints(vault: &Vault, rel: &str, from: &EntityRef, to: &EntityRef) -> Result<()> {
    let decl = check_relation(vault, rel)?;
    if let Some(f) = &decl.from
        && !f.contains(&from.kind)
    {
        bail!(
            "bad-endpoint: `{rel}` edges start from {f:?}, not from kind `{}`",
            from.kind
        );
    }
    if let Some(t) = &decl.to
        && !t.contains(&to.kind)
    {
        bail!(
            "bad-endpoint: `{rel}` edges point at {t:?}, not at kind `{}`",
            to.kind
        );
    }
    Ok(())
}

fn edge_list(vault: &Vault, source: &EntityRef, rel: &str) -> Vec<String> {
    vault
        .get(source)
        .map(|e| {
            e.fm.relations
                .get(rel)
                .into_iter()
                .flatten()
                .filter_map(|en| EntityRef::parse(en.target()).ok())
                .map(|r| elide(&r))
                .collect()
        })
        .unwrap_or_default()
}

// ───────────────────────────── operations ─────────────────────────────

#[derive(Default, serde::Deserialize)]
pub struct AddSpec {
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub namespace: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub lifecycle: Option<String>,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub relations: BTreeMap<String, Vec<RelEntry>>,
    #[serde(default)]
    pub source: Vec<String>,
}

pub fn add(vault: &Vault, spec: AddSpec) -> Result<Outcome> {
    let ns = spec.namespace.as_deref().unwrap_or("default");
    let eref = EntityRef::parse(&format!(
        "{}:{}/{}",
        nfc(&spec.kind),
        nfc(ns),
        nfc(&spec.name)
    ))
    .map_err(|e| anyhow!(e))?;
    if !vault.schema.kinds.contains_key(&eref.kind) {
        bail!(
            "kind `{}` is not declared (see `laplace query schema`, or `laplace schema add-kind`)",
            eref.kind
        );
    }
    if vault.get(&eref).is_some() {
        bail!(
            "{} already exists — did you `laplace query search {}` first?",
            eref,
            eref.name
        );
    }
    let mut edge_count = 0;
    for (rel, entries) in &spec.relations {
        for entry in entries {
            let target = EntityRef::parse(entry.target()).map_err(|e| anyhow!(e))?;
            resolve(vault, entry.target())?;
            check_endpoints(vault, rel, &eref, &target)?;
            edge_count += 1;
        }
    }
    let mut nudge = String::new();
    if spec.body.trim().is_empty() {
        nudge = match vault
            .schema
            .kinds
            .get(&eref.kind)
            .and_then(|k| k.description.as_ref())
        {
            Some(guide) => format!(
                "\n  warning: no body — the body is the entity's prose. Guide for `{}`: {}",
                eref.kind, guide
            ),
            None => "\n  warning: no body — the body is the entity's prose".into(),
        };
    }
    let fm = FrontMatter {
        title: spec.title,
        tags: spec.tags,
        lifecycle: spec.lifecycle,
        relations: spec.relations,
        source: spec.source,
        kind: None,
        name: None,
        extra: BTreeMap::new(),
    };
    let rel_path = entity_path(&eref);
    let content = render_entity(&fm, &spec.body);
    commit(
        vault,
        vec![Change::Write {
            rel: rel_path.clone(),
            content,
        }],
    )?;
    Ok(Outcome {
        message: format!(
            "added {} at {} ({} edges){nudge}",
            eref,
            vault.dir.join(&rel_path).display(),
            edge_count
        ),
        json: json!({ "op": "add", "ref": eref.to_string(), "path": rel_path, "edges": edge_count }),
    })
}

pub fn link(
    vault: &Vault,
    from: &str,
    rel: &str,
    to: &str,
    note: Option<String>,
) -> Result<Outcome> {
    let from = resolve(vault, from)?;
    let to = resolve(vault, to)?;
    let decl = check_relation(vault, rel)?;
    let symmetric = decl.symmetric;
    check_endpoints(vault, rel, &from, &to)?;

    let exists_on = |src: &EntityRef, dst: &EntityRef| {
        vault.get(src).is_some_and(|e| {
            e.fm.relations.get(rel).into_iter().flatten().any(|en| {
                EntityRef::parse(en.target())
                    .map(|r| &r == dst)
                    .unwrap_or(false)
            })
        })
    };
    if exists_on(&from, &to) {
        return Ok(Outcome {
            message: format!(
                "no-op: {} --{rel}--> {} already declared",
                elide(&from),
                elide(&to)
            ),
            json: json!({ "op": "link", "noop": true }),
        });
    }
    if symmetric && exists_on(&to, &from) {
        return Ok(Outcome {
            message: format!(
                "no-op: `{rel}` is symmetric and already declared on {} — one side suffices",
                elide(&to)
            ),
            json: json!({ "op": "link", "noop": true }),
        });
    }

    let mut entity = vault.get(&from).unwrap().clone();
    let entry = match note {
        None => RelEntry::Bare(elide(&to)),
        Some(n) => RelEntry::Object {
            r#ref: elide(&to),
            attrs: BTreeMap::from([("note".to_string(), serde_norway::Value::String(n))]),
        },
    };
    entity
        .fm
        .relations
        .entry(rel.to_string())
        .or_default()
        .push(entry);
    let content = render_entity(&entity.fm, &entity.body);
    commit(
        vault,
        vec![Change::Write {
            rel: entity.file.clone(),
            content,
        }],
    )?;

    let mut list = edge_list(vault, &from, rel);
    list.push(elide(&to));
    Ok(Outcome {
        message: format!(
            "linked: {} --{rel}--> {}\n  now {rel}: [{}]",
            elide(&from),
            elide(&to),
            list.join(", ")
        ),
        json: json!({ "op": "link", "from": from.to_string(), "rel": rel, "to": to.to_string(), "now": list }),
    })
}

pub fn unlink(vault: &Vault, from: &str, rel: &str, to: &str) -> Result<Outcome> {
    let from = resolve(vault, from)?;
    let to = resolve(vault, to)?;
    let symmetric = check_relation(vault, rel)?.symmetric;

    let take_from = |src: &EntityRef, dst: &EntityRef| -> Option<Entity> {
        let e = vault.get(src)?;
        let entries = e.fm.relations.get(rel)?;
        let keep: Vec<RelEntry> = entries
            .iter()
            .filter(|en| {
                EntityRef::parse(en.target())
                    .map(|r| &r != dst)
                    .unwrap_or(true)
            })
            .cloned()
            .collect();
        if keep.len() == entries.len() {
            return None;
        }
        let mut e = e.clone();
        if keep.is_empty() {
            e.fm.relations.remove(rel);
        } else {
            e.fm.relations.insert(rel.to_string(), keep);
        }
        Some(e)
    };
    let mutated = take_from(&from, &to)
        .or_else(|| {
            if symmetric {
                take_from(&to, &from)
            } else {
                None
            }
        })
        .ok_or_else(|| {
            anyhow!(
                "no edge {} --{rel}--> {} to remove",
                elide(&from),
                elide(&to)
            )
        })?;
    let content = render_entity(&mutated.fm, &mutated.body);
    let file = mutated.file.clone();
    commit(vault, vec![Change::Write { rel: file, content }])?;
    Ok(Outcome {
        message: format!("unlinked: {} --{rel}--> {}", elide(&from), elide(&to)),
        json: json!({ "op": "unlink" }),
    })
}

#[derive(Default)]
pub struct UpdateSpec {
    pub title: Option<String>,
    pub clear_title: bool,
    pub lifecycle: Option<String>,
    pub clear_lifecycle: bool,
    pub add_tags: Vec<String>,
    pub remove_tags: Vec<String>,
    pub set: Vec<(String, String)>,
    pub unset: Vec<String>,
    pub body: Option<String>,
}

pub fn update(vault: &Vault, r: &str, spec: UpdateSpec) -> Result<Outcome> {
    let eref = resolve(vault, r)?;
    let mut e = vault.get(&eref).unwrap().clone();
    if spec.clear_title {
        e.fm.title = None;
    }
    if let Some(t) = spec.title {
        e.fm.title = Some(t);
    }
    if spec.clear_lifecycle {
        e.fm.lifecycle = None;
    }
    if let Some(l) = spec.lifecycle {
        e.fm.lifecycle = Some(l);
    }
    for t in spec.add_tags {
        if !e.fm.tags.contains(&t) {
            e.fm.tags.push(t);
        }
    }
    e.fm.tags.retain(|t| !spec.remove_tags.contains(t));
    for (k, v) in spec.set {
        if matches!(
            k.as_str(),
            "kind" | "name" | "relations" | "source" | "title" | "tags" | "lifecycle"
        ) {
            bail!("`{k}` has a dedicated flag or operation; --set is for free-form keys");
        }
        e.fm.extra.insert(k, serde_norway::Value::String(v));
    }
    for k in spec.unset {
        e.fm.extra.remove(&k);
    }
    if let Some(b) = spec.body {
        e.body = b;
    }
    let content = render_entity(&e.fm, &e.body);
    let file = e.file.clone();
    commit(vault, vec![Change::Write { rel: file, content }])?;
    Ok(Outcome {
        message: format!("updated {eref}"),
        json: json!({ "op": "update", "ref": eref.to_string() }),
    })
}

pub fn remove(vault: &Vault, r: &str) -> Result<Outcome> {
    let eref = resolve(vault, r)?;
    let mut inbound: Vec<String> = Vec::new();
    for e in &vault.entities {
        if e.eref == eref {
            continue;
        }
        for (rel, entries) in &e.fm.relations {
            for en in entries {
                if EntityRef::parse(en.target()).ok().as_ref() == Some(&eref) {
                    inbound.push(format!("{} --{rel}--> it", elide(&e.eref)));
                }
            }
        }
    }
    if !inbound.is_empty() {
        bail!(
            "refused: {} inbound refs would dangle — unlink them first:\n  {}",
            inbound.len(),
            inbound.join("\n  ")
        );
    }
    let file = vault.get(&eref).unwrap().file.clone();
    commit(vault, vec![Change::Delete { rel: file }])?;
    Ok(Outcome {
        message: format!("removed {eref}"),
        json: json!({ "op": "remove", "ref": eref.to_string() }),
    })
}

pub fn rename(vault: &Vault, r: &str, new_name: &str, new_ns: Option<&str>) -> Result<Outcome> {
    let old = resolve(vault, r)?;
    let new = EntityRef::new(&old.kind, new_ns.unwrap_or(&old.ns), new_name);
    EntityRef::parse(&format!("{}:{}/{}", new.kind, new.ns, new.name)).map_err(|e| anyhow!(e))?;
    if new == old {
        bail!("rename to the same identity is a no-op");
    }
    if vault.get(&new).is_some() {
        bail!("{new} already exists");
    }

    let moved = vault.get(&old).unwrap().clone();
    let mut changes = vec![
        Change::Delete {
            rel: moved.file.clone(),
        },
        Change::Write {
            rel: entity_path(&new),
            content: render_entity(&moved.fm, &moved.body),
        },
    ];
    let mut rewritten_files = 0;
    for e in &vault.entities {
        if e.eref == old {
            continue;
        }
        let mut touched = false;
        let mut clone = e.clone();
        for entries in clone.fm.relations.values_mut() {
            for entry in entries.iter_mut() {
                if EntityRef::parse(entry.target())
                    .map(|t| t == old)
                    .unwrap_or(false)
                {
                    let s = elide(&new);
                    match entry {
                        RelEntry::Bare(b) => *b = s,
                        RelEntry::Object { r#ref, .. } => *r#ref = s,
                    }
                    touched = true;
                }
            }
        }
        if touched {
            rewritten_files += 1;
            changes.push(Change::Write {
                rel: clone.file.clone(),
                content: render_entity(&clone.fm, &clone.body),
            });
        }
    }
    let mentions: usize = vault
        .entities
        .iter()
        .map(|e| e.body.matches(old.name.as_str()).count())
        .sum();
    commit(vault, changes)?;
    let mention_note = if mentions > 0 {
        format!(
            "\n  note: {mentions} prose mentions of `{}` in bodies were left untouched (prose is human domain) — review by hand",
            old.name
        )
    } else {
        String::new()
    };
    Ok(Outcome {
        message: format!(
            "renamed {old} → {new}; rewrote inbound refs in {rewritten_files} files{mention_note}"
        ),
        json: json!({
            "op": "rename", "from": old.to_string(), "to": new.to_string(),
            "rewritten_files": rewritten_files, "prose_mentions": mentions,
        }),
    })
}

/// The one-line summary used in op echoes.
pub fn summary_of(e: &Entity) -> String {
    first_sentence(&e.body)
}
