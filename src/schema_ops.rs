//! Constitutional operations (SPEC §2): parse → mutate → canonical re-render →
//! simulate → persist. schema.yaml is machine-owned; prose lives in fields.

use crate::model::{Entity, KindDecl, Propagation, RelEntry, RelationDecl, Schema};
use crate::ops::{Change, Outcome, entity_path, render_entity};
use crate::validate::{self, Severity};
use crate::vault::Vault;
use anyhow::{Result, anyhow, bail};
use serde_json::json;
use std::collections::BTreeSet;

/// Canonical schema.yaml rendering: stable field order, defaults elided.
pub fn render_schema(s: &Schema) -> String {
    use serde_norway::{Mapping, Value};
    let mut m = Mapping::new();
    let put = |m: &mut Mapping, k: &str, v: Value| {
        m.insert(Value::String(k.into()), v);
    };
    put(&mut m, "apiVersion", Value::String(s.api_version.clone()));
    put(&mut m, "name", Value::String(s.name.clone()));
    if let Some(t) = &s.title {
        put(&mut m, "title", Value::String(t.clone()));
    }
    if s.root != ".." {
        put(&mut m, "root", Value::String(s.root.clone()));
    }
    let str_list = |v: &[String]| Value::Sequence(v.iter().cloned().map(Value::String).collect());
    if !s.charter.is_empty() {
        put(&mut m, "charter", str_list(&s.charter));
    }
    if !s.ignore.is_empty() {
        put(&mut m, "ignore", str_list(&s.ignore));
    }
    if !s.exclusions.is_empty() {
        put(&mut m, "exclusions", str_list(&s.exclusions));
    }
    let mut kinds = Mapping::new();
    for (k, d) in &s.kinds {
        let mut km = Mapping::new();
        if let Some(desc) = &d.description {
            put(&mut km, "description", Value::String(desc.clone()));
        }
        kinds.insert(Value::String(k.clone()), Value::Mapping(km));
    }
    put(&mut m, "kinds", Value::Mapping(kinds));
    let mut rels = Mapping::new();
    for (r, d) in &s.relations {
        let mut rm = Mapping::new();
        if let Some(desc) = &d.description {
            put(&mut rm, "description", Value::String(desc.clone()));
        }
        put(
            &mut rm,
            "propagation",
            Value::String(d.propagation.as_str().into()),
        );
        if d.symmetric {
            put(&mut rm, "symmetric", Value::Bool(true));
        }
        if d.acyclic {
            put(&mut rm, "acyclic", Value::Bool(true));
        }
        if let Some(f) = &d.from {
            put(&mut rm, "from", str_list(f));
        }
        if let Some(t) = &d.to {
            put(&mut rm, "to", str_list(t));
        }
        rels.insert(Value::String(r.clone()), Value::Mapping(rm));
    }
    put(&mut m, "relations", Value::Mapping(rels));
    serde_norway::to_string(&m).expect("schema serializes")
}

/// Simulate a schema change (plus optional entity rewrites), abort on new errors,
/// then persist schema.yaml and the entity changes.
fn commit(
    vault: &Vault,
    new_schema: Schema,
    entity_changes: Vec<Change>,
    mutated_entities: Vec<Entity>,
    deleted: &BTreeSet<std::path::PathBuf>,
) -> Result<()> {
    let baseline: BTreeSet<String> = validate::run(vault)
        .diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.render())
        .collect();
    let mut entities: Vec<Entity> = vault
        .entities
        .iter()
        .filter(|e| {
            !deleted.contains(&e.file) && !mutated_entities.iter().any(|m| m.eref == e.eref)
        })
        .cloned()
        .collect();
    entities.extend(mutated_entities);
    let sim = Vault {
        dir: vault.dir.clone(),
        project_root: vault.project_root.clone(),
        schema: new_schema.clone(),
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
    if !new_errors.is_empty() {
        bail!(
            "refused — the constitutional change would introduce errors:\n{}",
            new_errors.join("\n")
        );
    }
    // Persist: schema first (temp+rename), then entity changes.
    let target = vault.dir.join("schema.yaml");
    let tmp = vault.dir.join(".laplace-tmp-schema.yaml");
    std::fs::write(&tmp, render_schema(&new_schema))?;
    std::fs::rename(tmp, target)?;
    for c in &entity_changes {
        match c {
            Change::Write { rel, content } => {
                let t = vault.dir.join(rel);
                if let Some(p) = t.parent() {
                    std::fs::create_dir_all(p)?;
                }
                std::fs::write(&t, content)?;
            }
            Change::Delete { rel } => {
                std::fs::remove_file(vault.dir.join(rel))?;
            }
        }
    }
    Ok(())
}

pub fn add_kind(vault: &Vault, name: &str, description: Option<String>) -> Result<Outcome> {
    if vault.schema.kinds.contains_key(name) {
        bail!("kind `{name}` already declared");
    }
    let mut s = vault.schema.clone();
    s.kinds.insert(name.to_string(), KindDecl { description });
    commit(vault, s, vec![], vec![], &BTreeSet::new())?;
    Ok(Outcome {
        message: format!(
            "declared kind `{name}` — cite the charter question it serves in your commit"
        ),
        json: json!({ "op": "schema.add-kind", "kind": name }),
    })
}

#[derive(Default)]
pub struct RelationSpec {
    pub description: String,
    pub propagation: Option<Propagation>,
    pub symmetric: bool,
    pub acyclic: bool,
    pub from: Option<Vec<String>>,
    pub to: Option<Vec<String>>,
}

pub fn add_relation(vault: &Vault, name: &str, spec: RelationSpec) -> Result<Outcome> {
    if vault.schema.relations.contains_key(name) {
        bail!("relation `{name}` already declared");
    }
    if spec.description.trim().is_empty() {
        bail!(
            "a relation must state its reading direction (\"A {name} B —— A 是…，B 是…\") — direction confusion corrupts silently"
        );
    }
    let mut s = vault.schema.clone();
    s.relations.insert(
        name.to_string(),
        RelationDecl {
            description: Some(spec.description),
            propagation: spec.propagation.unwrap_or_default(),
            symmetric: spec.symmetric,
            acyclic: spec.acyclic,
            from: spec.from,
            to: spec.to,
        },
    );
    commit(vault, s, vec![], vec![], &BTreeSet::new())?;
    Ok(Outcome {
        message: format!("declared relation `{name}` — cite the charter question it serves"),
        json: json!({ "op": "schema.add-relation", "relation": name }),
    })
}

/// `laplace schema set (kinds|relations).<name>.<field> <value>`
pub fn set(vault: &Vault, path: &str, value: &str) -> Result<Outcome> {
    let parts: Vec<&str> = path.splitn(3, '.').collect();
    let [section, name, field] = parts.as_slice() else {
        bail!("path is (kinds|relations).<name>.<field>");
    };
    let mut s = vault.schema.clone();
    match *section {
        "kinds" => {
            let d = s
                .kinds
                .get_mut(*name)
                .ok_or_else(|| anyhow!("no kind `{name}`"))?;
            match *field {
                "description" => d.description = Some(value.to_string()),
                f => bail!("kinds have no settable field `{f}` (only description)"),
            }
        }
        "relations" => {
            let d = s
                .relations
                .get_mut(*name)
                .ok_or_else(|| anyhow!("no relation `{name}`"))?;
            match *field {
                "description" => d.description = Some(value.to_string()),
                "propagation" => {
                    d.propagation = serde_norway::from_str(value)
                        .map_err(|_| anyhow!("propagation is to-source|to-target|both|none"))?
                }
                "symmetric" => d.symmetric = value.parse().map_err(|_| anyhow!("true|false"))?,
                "acyclic" => d.acyclic = value.parse().map_err(|_| anyhow!("true|false"))?,
                "from" => d.from = Some(value.split(',').map(|s| s.trim().to_string()).collect()),
                "to" => d.to = Some(value.split(',').map(|s| s.trim().to_string()).collect()),
                f => bail!("relations have no settable field `{f}`"),
            }
        }
        s => bail!("unknown section `{s}` — kinds or relations"),
    }
    commit(vault, s, vec![], vec![], &BTreeSet::new())?;
    Ok(Outcome {
        message: format!("set {path} = {value}"),
        json: json!({ "op": "schema.set", "path": path, "value": value }),
    })
}

pub fn rename_relation(vault: &Vault, old: &str, new: &str) -> Result<Outcome> {
    if !vault.schema.relations.contains_key(old) {
        bail!("no relation `{old}`");
    }
    if vault.schema.relations.contains_key(new) {
        bail!("relation `{new}` already exists — merging types is a judgment call, do it by hand");
    }
    let mut s = vault.schema.clone();
    let decl = s.relations.remove(old).unwrap();
    s.relations.insert(new.to_string(), decl);

    let mut changes = Vec::new();
    let mut mutated = Vec::new();
    for e in &vault.entities {
        if let Some(entries) = e.fm.relations.get(old) {
            let mut clone = e.clone();
            let entries = entries.clone();
            clone.fm.relations.remove(old);
            clone.fm.relations.insert(new.to_string(), entries);
            changes.push(Change::Write {
                rel: clone.file.clone(),
                content: render_entity(&clone.fm, &clone.body),
            });
            mutated.push(clone);
        }
    }
    let n = mutated.len();
    commit(vault, s, changes, mutated, &BTreeSet::new())?;
    Ok(Outcome {
        message: format!("renamed relation `{old}` → `{new}`; rewrote {n} entity files"),
        json: json!({ "op": "schema.rename-relation", "from": old, "to": new, "rewritten": n }),
    })
}

pub fn rename_kind(vault: &Vault, old: &str, new: &str) -> Result<Outcome> {
    if !vault.schema.kinds.contains_key(old) {
        bail!("no kind `{old}`");
    }
    if vault.schema.kinds.contains_key(new) {
        bail!("kind `{new}` already exists — merging kinds is a judgment call, do it by hand");
    }
    let mut s = vault.schema.clone();
    let decl = s.kinds.remove(old).unwrap();
    s.kinds.insert(new.to_string(), decl);
    for d in s.relations.values_mut() {
        for list in [&mut d.from, &mut d.to].into_iter().flatten() {
            for k in list.iter_mut() {
                if k == old {
                    *k = new.to_string();
                }
            }
        }
    }

    let mut changes = Vec::new();
    let mut mutated = Vec::new();
    let mut deleted = BTreeSet::new();
    for e in &vault.entities {
        let mut clone = e.clone();
        let mut touched = false;
        // Rewrite every ref whose kind is `old`, wherever it appears.
        for entries in clone.fm.relations.values_mut() {
            for entry in entries.iter_mut() {
                if let Ok(mut t) = crate::model::EntityRef::parse(entry.target())
                    && t.kind == old
                {
                    t.kind = new.to_string();
                    let s = crate::ops::elide(&t);
                    match entry {
                        RelEntry::Bare(b) => *b = s,
                        RelEntry::Object { r#ref, .. } => *r#ref = s,
                    }
                    touched = true;
                }
            }
        }
        // Entities of the old kind move to the new directory.
        if clone.eref.kind == old {
            deleted.insert(clone.file.clone());
            clone.eref.kind = new.to_string();
            clone.file = entity_path(&clone.eref);
            touched = true;
        }
        if touched {
            changes.push(Change::Write {
                rel: clone.file.clone(),
                content: render_entity(&clone.fm, &clone.body),
            });
            mutated.push(clone);
        }
    }
    for rel in &deleted {
        changes.push(Change::Delete { rel: rel.clone() });
    }
    let n = mutated.len();
    commit(vault, s, changes, mutated, &deleted)?;
    // The old kind directory may now be empty; tidy it quietly.
    let _ = std::fs::remove_dir(vault.dir.join(old));
    Ok(Outcome {
        message: format!("renamed kind `{old}` → `{new}`; moved/rewrote {n} entity files"),
        json: json!({ "op": "schema.rename-kind", "from": old, "to": new, "rewritten": n }),
    })
}
