//! The entity-map skill: embedded in the binary (single source, version-locked
//! to the tool behavior it documents), printable and installable into harness
//! skill directories (`<dir>/entity-map/SKILL.md` — the convention Claude Code
//! and bingo share).

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

pub const SKILL_NAME: &str = "entity-map";
pub const SKILL_TEXT: &str = include_str!("../skill/entity-map/SKILL.md");

/// The compact distillation served as MCP `instructions` — the zero-install
/// channel for clients that inject it (Claude Code does; see bingo#77).
pub const MCP_INSTRUCTIONS: &str = "\
Laplace serves entity maps (vaults): the authoritative picture of what exists \
in a project and how things connect. Query, don't guess: laplace_schema shows \
the vocabulary (kinds, relation reading-directions) — consult it before \
writing; laplace_search resolves names before you add or guess a ref; \
laplace_impact answers \"what does changing X touch\" (a candidate set — trust \
decays with distance); laplace_trace answers \"how are these connected\"; \
laplace_get / laplace_neighbors give detail and context. If your work adds, \
renames, removes, or re-relates an entity, update the map in the same turn via \
laplace_add / laplace_link / laplace_update — after direct file edits run \
laplace_validate. Several vaults may be loaded: laplace_vaults lists them; \
pass `vault` to select.";

/// Candidate install locations that already exist on this machine.
pub fn detect_targets(cwd: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        let claude = home.join(".claude").join("skills");
        if claude.parent().is_some_and(|p| p.is_dir()) {
            out.push(claude);
        }
        let config = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"));
        let bingo = config.join("bingo").join("skills");
        if bingo.parent().is_some_and(|p| p.is_dir()) {
            out.push(bingo);
        }
    }
    for project_dir in [cwd.join(".claude"), cwd.join(".bingo")] {
        if project_dir.is_dir() {
            out.push(project_dir.join("skills"));
        }
    }
    out
}

/// Write `<dir>/entity-map/SKILL.md`, overwriting an older copy.
pub fn install_into(dir: &Path) -> Result<PathBuf> {
    let skill_dir = dir.join(SKILL_NAME);
    std::fs::create_dir_all(&skill_dir)
        .with_context(|| format!("creating {}", skill_dir.display()))?;
    let path = skill_dir.join("SKILL.md");
    std::fs::write(&path, SKILL_TEXT).with_context(|| format!("writing {}", path.display()))?;
    Ok(path)
}

pub fn install(cwd: &Path, to: Option<&Path>) -> Result<Vec<PathBuf>> {
    let targets = match to {
        Some(dir) => vec![dir.to_path_buf()],
        None => detect_targets(cwd),
    };
    if targets.is_empty() {
        bail!(
            "no harness skill directories detected (looked for ~/.claude, ~/.config/bingo, ./.claude, ./.bingo) — pass --to DIR"
        );
    }
    targets.iter().map(|d| install_into(d)).collect()
}
