//! Vault discovery and loading (SPEC §1.1–§1.2). Loading is total: structural
//! problems become diagnostics, never panics, so validate can report them all.

use crate::model::{API_VERSION, Entity, EntityRef, FrontMatter, Schema, nfc};
use crate::validate::{Diagnostic, Severity};
use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub struct Vault {
    /// The directory containing schema.yaml.
    pub dir: PathBuf,
    /// What source/ignore globs resolve against: dir joined with schema.root.
    pub project_root: PathBuf,
    pub schema: Schema,
    pub entities: Vec<Entity>,
    /// Structure-layer diagnostics collected during load.
    pub load_diags: Vec<Diagnostic>,
}

/// Find the vault: `--vault DIR` (must contain schema.yaml), else upward from
/// `start` for the first `laplace/schema.yaml`.
pub fn discover(start: &Path, explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(dir) = explicit {
        if dir.join("schema.yaml").is_file() {
            return Ok(dir.to_path_buf());
        }
        bail!("--vault {}: no schema.yaml there", dir.display());
    }
    let mut cur = Some(start);
    while let Some(dir) = cur {
        let candidate = dir.join("laplace");
        if candidate.join("schema.yaml").is_file() {
            return Ok(candidate);
        }
        cur = dir.parent();
    }
    bail!(
        "no vault found: no `laplace/schema.yaml` from {} upward (create one with `laplace init`, or pass --vault)",
        start.display()
    );
}

pub fn load(dir: &Path) -> Result<Vault> {
    let schema_path = dir.join("schema.yaml");
    let schema_text = std::fs::read_to_string(&schema_path)
        .with_context(|| format!("reading {}", schema_path.display()))?;
    let schema: Schema = serde_norway::from_str(&schema_text)
        .with_context(|| format!("{}: schema.yaml does not parse", schema_path.display()))?;
    if !schema.api_version.starts_with("laplace/v1") {
        bail!(
            "{}: apiVersion `{}` is not `{}` — this reader refuses unknown majors",
            schema_path.display(),
            schema.api_version,
            API_VERSION
        );
    }
    let project_root = normalize(&dir.join(&schema.root));

    let mut entities = Vec::new();
    let mut diags = Vec::new();
    let mut by_ref: BTreeMap<EntityRef, PathBuf> = BTreeMap::new();
    let mut files = Vec::new();
    collect_md(dir, dir, &mut files, &mut diags);
    files.sort();

    for file in files {
        let rel = file.strip_prefix(dir).unwrap().to_path_buf();
        let raw = match std::fs::read_to_string(&file) {
            Ok(t) => t,
            Err(e) => {
                diags.push(Diagnostic::err(
                    "unreadable",
                    Some(rel.clone()),
                    format!("cannot read: {e}"),
                ));
                continue;
            }
        };
        let (entity, entity_diags) = parse_entity(rel.clone(), raw);
        diags.extend(entity_diags);
        let Some(entity) = entity else { continue };
        if let Some(prev) = by_ref.get(&entity.eref) {
            // Only reachable via NFC collision: two byte-distinct paths, one identity.
            diags.push(Diagnostic {
                severity: Severity::Error,
                code: "duplicate-entity",
                file: Some(rel),
                line: None,
                entity: Some(entity.eref.to_string()),
                path: None,
                message: format!(
                    "normalizes to the same identity as {} (Unicode NFC)",
                    prev.display()
                ),
                suggestion: None,
            });
            continue;
        }
        by_ref.insert(entity.eref.clone(), entity.file.clone());
        entities.push(entity);
    }

    if entities.is_empty() {
        diags.push(Diagnostic {
            severity: Severity::Warning,
            code: "empty-vault",
            file: None,
            line: None,
            entity: None,
            path: None,
            message: "the vault has no entities".into(),
            suggestion: None,
        });
    }

    Ok(Vault {
        dir: dir.to_path_buf(),
        project_root,
        schema,
        entities,
        load_diags: diags,
    })
}

impl Vault {
    pub fn get(&self, eref: &EntityRef) -> Option<&Entity> {
        self.entities.iter().find(|e| &e.eref == eref)
    }

    /// Resolve user input to an entity ref: exact canonical match after parse.
    pub fn resolve(&self, input: &str) -> Result<EntityRef> {
        let r = EntityRef::parse(input).map_err(|e| anyhow::anyhow!(e))?;
        if self.get(&r).is_some() {
            Ok(r)
        } else {
            bail!(
                "no such entity: {r} (try `laplace query search {}`)",
                r.name
            )
        }
    }
}

/// Parse one entity file from its vault-relative path and raw text. Total:
/// problems come back as diagnostics; `None` means the file yields no entity.
/// Shared by `load` and by the write operations' pre-commit simulation.
pub fn parse_entity(rel: PathBuf, raw: String) -> (Option<Entity>, Vec<Diagnostic>) {
    let mut diags = Vec::new();
    let comps: Vec<String> = rel
        .iter()
        .map(|c| c.to_string_lossy().into_owned())
        .collect();
    let (kind, ns, stem_) = match comps.len() {
        2 => (comps[0].clone(), "default".to_string(), stem(&comps[1])),
        3 => (comps[0].clone(), comps[1].clone(), stem(&comps[2])),
        n => {
            diags.push(Diagnostic::err(
                "bad-layout",
                Some(rel),
                format!(
                    "entity files live at <kind>/<name>.md or <kind>/<namespace>/<name>.md, not {n} levels deep"
                ),
            ));
            return (None, diags);
        }
    };
    let eref = EntityRef::new(&kind, &ns, &stem_);

    let (fm, body) = match split_frontmatter(&raw) {
        Ok((fm_text, body)) => {
            let fm: FrontMatter = if fm_text.trim().is_empty() {
                FrontMatter::default()
            } else {
                match serde_norway::from_str(fm_text) {
                    Ok(fm) => fm,
                    Err(e) => {
                        let line = e.location().map(|l| l.line() + 1);
                        diags.push(Diagnostic {
                            severity: Severity::Error,
                            code: "bad-frontmatter",
                            file: Some(rel),
                            line,
                            entity: Some(eref.to_string()),
                            path: None,
                            message: format!("frontmatter does not parse: {e}"),
                            suggestion: None,
                        });
                        return (None, diags);
                    }
                }
            };
            (fm, body.to_string())
        }
        Err(msg) => {
            diags.push(Diagnostic::err("bad-frontmatter", Some(rel), msg));
            return (None, diags);
        }
    };

    if fm.kind.is_some() || fm.name.is_some() {
        diags.push(Diagnostic {
            severity: Severity::Error,
            code: "identity-in-frontmatter",
            file: Some(rel.clone()),
            line: None,
            entity: Some(eref.to_string()),
            path: None,
            message:
                "path is identity: kind and name derive from the file path and must not appear in frontmatter"
                    .into(),
            suggestion: Some("delete the kind:/name: keys".into()),
        });
    }
    if body.trim().is_empty() {
        diags.push(Diagnostic {
            severity: Severity::Warning,
            code: "empty-body",
            file: Some(rel.clone()),
            line: None,
            entity: Some(eref.to_string()),
            path: None,
            message: "no description: the body is the entity's prose".into(),
            suggestion: None,
        });
    }
    (
        Some(Entity {
            eref,
            file: rel,
            fm,
            body,
            raw,
        }),
        diags,
    )
}

fn collect_md(root: &Path, dir: &Path, out: &mut Vec<PathBuf>, diags: &mut Vec<Diagnostic>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            diags.push(Diagnostic::err(
                "unreadable",
                Some(dir.strip_prefix(root).unwrap_or(dir).to_path_buf()),
                format!("cannot list: {e}"),
            ));
            return;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            collect_md(root, &path, out, diags);
        } else if name.ends_with(".md") {
            if path.parent() == Some(root) {
                diags.push(Diagnostic::err(
                    "bad-layout",
                    Some(PathBuf::from(&name)),
                    "markdown at the vault root: entities live under <kind>/".into(),
                ));
            } else {
                out.push(path);
            }
        }
    }
}

fn stem(file_name: &str) -> String {
    nfc(file_name.strip_suffix(".md").unwrap_or(file_name))
}

/// Split `---\n<yaml>\n---\n<body>`. A file not starting with `---` is all body.
fn split_frontmatter(raw: &str) -> Result<(&str, &str), String> {
    let Some(rest) = raw.strip_prefix("---\n").or(raw.strip_prefix("---\r\n")) else {
        return Ok(("", raw));
    };
    // Empty frontmatter: the closing fence follows the opening one directly.
    if let Some(after) = rest.strip_prefix("---")
        && (after.is_empty() || after.starts_with('\n') || after.starts_with("\r\n"))
    {
        return Ok(("", after.strip_prefix('\n').unwrap_or(after)));
    }
    for (i, _) in rest.match_indices("\n---") {
        let after = &rest[i + 4..];
        if after.is_empty() || after.starts_with('\n') || after.starts_with("\r\n") {
            let body = after.strip_prefix('\n').unwrap_or(after);
            return Ok((&rest[..i], body));
        }
    }
    Err("frontmatter fence `---` is never closed".into())
}

fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            std::path::Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            std::path::Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontmatter_split() {
        let (fm, body) = split_frontmatter("---\ntags: [a]\n---\nbody here").unwrap();
        assert_eq!(fm, "tags: [a]");
        assert_eq!(body, "body here");
        let (fm, body) = split_frontmatter("no fence").unwrap();
        assert_eq!(fm, "");
        assert_eq!(body, "no fence");
        let (fm, body) = split_frontmatter("---\n---\nempty fm").unwrap();
        assert_eq!(fm, "");
        assert_eq!(body, "empty fm");
        assert!(split_frontmatter("---\nnever closed").is_err());
    }

    #[test]
    fn flow_ref_with_colon_parses_as_string() {
        let fm: FrontMatter =
            serde_norway::from_str("relations:\n  隶属: [location:东海龙宫]\n").unwrap();
        assert_eq!(fm.relations["隶属"][0].target(), "location:东海龙宫");
    }
}
