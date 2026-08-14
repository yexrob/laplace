//! The summary projection (SPEC §7): the block a harness injects into agent
//! context. Tiered to a token budget — T0 (header + charter + kinds + the
//! discipline footer) always emits; relation-types, recent changes, and
//! per-kind name lists join as the budget allows, largest lists truncating
//! first with explicit `…+N more` markers.

use crate::model::is_wide;
use crate::vault::Vault;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::process::Command;

pub const DEFAULT_BUDGET: usize = 1200;

/// CJK-aware token estimate: ASCII ≈ 4 chars/token, CJK ≈ 1 char/token,
/// the rest ≈ 2 (ASCII-calibrated ÷4 heuristics undercount CJK badly).
pub fn estimate_tokens(s: &str) -> usize {
    let (mut ascii, mut cjk, mut other) = (0usize, 0usize, 0usize);
    for c in s.chars() {
        if c.is_ascii() {
            ascii += 1;
        } else if is_wide(c) {
            cjk += 1;
        } else {
            other += 1;
        }
    }
    ascii.div_ceil(4) + cjk + other.div_ceil(2)
}

pub struct Summary {
    pub text: String,
    pub tokens: usize,
    /// Highest tier that made it in: 0..=3.
    pub tier: u8,
}

pub fn render(vault: &Vault, budget: usize) -> Summary {
    let mut kind_counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut names_by_kind: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut rel_counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut edge_total = 0usize;
    for e in &vault.entities {
        *kind_counts.entry(e.eref.kind.as_str()).or_default() += 1;
        names_by_kind
            .entry(e.eref.kind.as_str())
            .or_default()
            .push(e.eref.name.as_str());
        for (rel, entries) in &e.fm.relations {
            *rel_counts.entry(rel.as_str()).or_default() += entries.len();
            edge_total += entries.len();
        }
    }
    for names in names_by_kind.values_mut() {
        names.sort();
    }

    let project = vault
        .schema
        .title
        .clone()
        .unwrap_or_else(|| vault.schema.name.clone());
    let updated = vault_updated(vault);
    let header = format!(
        "<laplace-map project=\"{project}\" entities=\"{}\" relations=\"{edge_total}\"{}>",
        vault.entities.len(),
        updated
            .as_deref()
            .map(|d| format!(" updated=\"{d}\""))
            .unwrap_or_default(),
    );
    let charter = (!vault.schema.charter.is_empty())
        .then(|| format!("charter: {}", vault.schema.charter.join(" / ")));
    let kinds_line = format!(
        "kinds: {}",
        kind_counts
            .iter()
            .map(|(k, c)| format!("{k}({c})"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let footer = "This map is authoritative. Details: laplace query / MCP tools; do not guess beyond it.\n\
                  If your work touches an entity (add/rename/remove/re-relate), update the map in the same turn via add/link/update.\n\
                  </laplace-map>";

    // T1: relation types by frequency.
    let mut rels: Vec<(&str, usize)> = rel_counts.into_iter().collect();
    rels.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    let t1 = (!rels.is_empty()).then(|| {
        format!(
            "relation-types: {}",
            rels.iter()
                .map(|(r, c)| format!("{r}({c})"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    });

    // T2: recent vault commits (omitted without git).
    let t2 = recent_changes(vault).map(|lines| format!("recent:\n{lines}"));

    // T3: per-kind name lists, truncatable.
    let assemble = |name_cap: &BTreeMap<&str, usize>, tier: u8| -> String {
        let mut out = vec![header.clone()];
        if let Some(c) = &charter {
            out.push(c.clone());
        }
        out.push(kinds_line.clone());
        if tier >= 1
            && let Some(t) = &t1
        {
            out.push(t.clone());
        }
        if tier >= 3 {
            for (kind, names) in &names_by_kind {
                let cap = name_cap.get(kind).copied().unwrap_or(names.len());
                if cap == 0 {
                    continue;
                }
                let shown = &names[..cap.min(names.len())];
                let more = names.len() - shown.len();
                let suffix = if more > 0 {
                    format!(", …+{more} more")
                } else {
                    String::new()
                };
                out.push(format!("{kind}: {}{suffix}", shown.join(", ")));
            }
        }
        if tier >= 2
            && let Some(t) = &t2
        {
            out.push(t.clone());
        }
        out.push(footer.to_string());
        out.join("\n")
    };

    // Fit: full T3 → shrink the largest list one name at a time → T2 → T1 → T0.
    let mut caps: BTreeMap<&str, usize> =
        names_by_kind.iter().map(|(k, v)| (*k, v.len())).collect();
    for tier in [3u8, 2, 1, 0] {
        loop {
            let text = assemble(&caps, tier);
            let tokens = estimate_tokens(&text);
            if tokens <= budget || tier == 0 {
                return Summary { text, tokens, tier };
            }
            if tier < 3 {
                break; // this tier has nothing left to shrink; drop lower.
            }
            // Shrink the largest remaining list; when all are empty, leave T3.
            let Some((&kind, _)) = caps
                .iter()
                .filter(|(_, c)| **c > 0)
                .max_by_key(|(_, c)| **c)
            else {
                break;
            };
            *caps.get_mut(kind).unwrap() -= 1;
        }
    }
    unreachable!("tier 0 always returns");
}

pub fn to_json(s: &Summary) -> Value {
    json!({ "text": s.text, "tokens_estimated": s.tokens, "tier": s.tier })
}

fn git_out(vault: &Vault, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(&vault.dir)
        .args(args)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Last vault commit date (short), for title blocks and export.
pub fn vault_updated_date(vault: &Vault) -> Option<String> {
    vault_updated(vault)
}

fn vault_updated(vault: &Vault) -> Option<String> {
    git_out(
        vault,
        &["log", "-n1", "--format=%ad", "--date=short", "--", "."],
    )
}

fn recent_changes(vault: &Vault) -> Option<String> {
    let log = git_out(
        vault,
        &["log", "-n5", "--format=%ad %s", "--date=short", "--", "."],
    )?;
    Some(
        log.lines()
            .map(|l| format!("  {l}"))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cjk_counts_as_full_tokens() {
        assert_eq!(estimate_tokens("孙悟空大闹天宫再闹天宫"), 11);
        assert_eq!(estimate_tokens("abcdefgh"), 2);
    }
}
