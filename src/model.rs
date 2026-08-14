//! Core types: refs, the schema (constitution), entities. SPEC §1.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;
use unicode_normalization::UnicodeNormalization;

pub const API_VERSION: &str = "laplace/v1";
pub const DEFAULT_NS: &str = "default";

/// Canonical entity reference: `kind:namespace/name`. Components are NFC-normalized.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct EntityRef {
    pub kind: String,
    pub ns: String,
    pub name: String,
}

impl EntityRef {
    pub fn new(kind: &str, ns: &str, name: &str) -> Self {
        Self {
            kind: nfc(kind),
            ns: nfc(ns),
            name: nfc(name),
        }
    }

    /// Parse `kind:name` or `kind:namespace/name` (SPEC §1.3).
    pub fn parse(s: &str) -> Result<Self, String> {
        let (kind, rest) = s
            .split_once(':')
            .ok_or_else(|| format!("`{s}`: a ref is `kind:name` or `kind:namespace/name`"))?;
        let (ns, name) = match rest.split_once('/') {
            Some((ns, name)) => (ns, name),
            None => (DEFAULT_NS, rest),
        };
        for w in [kind, ns, name] {
            if !valid_word(w) {
                return Err(format!(
                    "`{s}`: `{w}` is not a valid word (no whitespace, `:` `/` `,` `[` `]` or quotes; non-empty)"
                ));
            }
        }
        Ok(Self::new(kind, ns, name))
    }
}

impl fmt::Display for EntityRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}/{}", self.kind, self.ns, self.name)
    }
}

pub fn nfc(s: &str) -> String {
    s.nfc().collect()
}

pub fn valid_word(w: &str) -> bool {
    !w.is_empty()
        && w.chars()
            .all(|c| !c.is_whitespace() && !matches!(c, ':' | '/' | ',' | '[' | ']' | '"' | '\''))
}

/// Where a change propagates along an edge `A --rel--> B` (SPEC §1.6).
#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Propagation {
    /// A change to B affects A (dependency). The default.
    #[default]
    ToSource,
    /// A change to A affects B (containment/appearance).
    ToTarget,
    Both,
    None,
}

impl Propagation {
    pub fn as_str(self) -> &'static str {
        match self {
            Propagation::ToSource => "to-source",
            Propagation::ToTarget => "to-target",
            Propagation::Both => "both",
            Propagation::None => "none",
        }
    }
}

#[derive(Deserialize, Default, Debug)]
pub struct KindDecl {
    #[serde(default)]
    pub description: Option<String>,
}

impl KindDecl {
    /// Convention (SPEC §1.5): first sentence = display label, rest = authoring guide.
    pub fn label(&self) -> Option<String> {
        self.description.as_deref().map(first_sentence)
    }
}

#[derive(Deserialize, Debug)]
pub struct RelationDecl {
    /// Required; must state the reading direction. Checked in validate for a
    /// better diagnostic than a serde error.
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub propagation: Propagation,
    #[serde(default)]
    pub symmetric: bool,
    #[serde(default)]
    pub from: Option<Vec<String>>,
    #[serde(default)]
    pub to: Option<Vec<String>>,
    #[serde(default)]
    pub acyclic: bool,
}

fn default_root() -> String {
    "..".into()
}

/// `schema.yaml` — the constitution (SPEC §1.5).
#[derive(Deserialize, Debug)]
pub struct Schema {
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    pub name: String,
    #[serde(default)]
    pub title: Option<String>,
    /// What source/ignore globs resolve against, relative to the vault dir.
    #[serde(default = "default_root")]
    pub root: String,
    #[serde(default)]
    pub charter: Vec<String>,
    #[serde(default)]
    pub ignore: Vec<String>,
    #[serde(default)]
    pub exclusions: Vec<String>,
    #[serde(default)]
    pub kinds: BTreeMap<String, KindDecl>,
    #[serde(default)]
    pub relations: BTreeMap<String, RelationDecl>,
}

/// One relation entry: a bare ref or an object with free edge attributes.
#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum RelEntry {
    Bare(String),
    Object {
        r#ref: String,
        #[serde(flatten)]
        attrs: BTreeMap<String, serde_norway::Value>,
    },
}

impl RelEntry {
    pub fn target(&self) -> &str {
        match self {
            RelEntry::Bare(s) => s,
            RelEntry::Object { r#ref, .. } => r#ref,
        }
    }

    pub fn note(&self) -> Option<&str> {
        match self {
            RelEntry::Bare(_) => None,
            RelEntry::Object { attrs, .. } => attrs.get("note").and_then(|v| v.as_str()),
        }
    }
}

/// Entity frontmatter — flat, machine-owned (SPEC §1.2). `kind`/`name` are
/// captured only to reject them: path is identity.
#[derive(Deserialize, Default, Debug)]
pub struct FrontMatter {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub lifecycle: Option<String>,
    #[serde(default)]
    pub relations: BTreeMap<String, Vec<RelEntry>>,
    #[serde(default)]
    pub source: Vec<String>,
    #[serde(default)]
    pub kind: Option<serde_norway::Value>,
    #[serde(default)]
    pub name: Option<serde_norway::Value>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_norway::Value>,
}

#[derive(Debug)]
pub struct Entity {
    pub eref: EntityRef,
    /// Vault-relative path.
    pub file: PathBuf,
    pub fm: FrontMatter,
    pub body: String,
    /// Raw file text, for line-number lookup in diagnostics.
    pub raw: String,
}

impl Entity {
    pub fn title(&self) -> &str {
        self.fm.title.as_deref().unwrap_or(&self.eref.name)
    }

    pub fn first_sentence(&self) -> String {
        first_sentence(&self.body)
    }

    /// Best-effort line number of the first line containing `needle`.
    pub fn line_of(&self, needle: &str) -> Option<usize> {
        self.raw
            .lines()
            .position(|l| l.contains(needle))
            .map(|i| i + 1)
    }
}

/// First sentence of a text, capped at 120 display cells (SPEC §1.2):
/// CJK terminators always end a sentence; ASCII `.` `!` `?` only before
/// whitespace or end, so `v1.2` survives.
pub fn first_sentence(text: &str) -> String {
    let text = text.trim_start();
    let mut out = String::new();
    let mut cells = 0usize;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\n' {
            break;
        }
        out.push(c);
        cells += if is_wide(c) { 2 } else { 1 };
        let terminal = matches!(c, '。' | '！' | '？')
            || (matches!(c, '.' | '!' | '?')
                && chars.peek().map(|n| n.is_whitespace()).unwrap_or(true));
        if terminal || cells >= 120 {
            break;
        }
    }
    out.trim_end().to_string()
}

pub fn is_wide(c: char) -> bool {
    matches!(c as u32,
        0x1100..=0x115F | 0x2E80..=0x303E | 0x3041..=0x33FF | 0x3400..=0x4DBF
        | 0x4E00..=0x9FFF | 0xA000..=0xA4CF | 0xAC00..=0xD7A3 | 0xF900..=0xFAFF
        | 0xFE30..=0xFE4F | 0xFF00..=0xFF60 | 0xFFE0..=0xFFE6 | 0x20000..=0x2FA1F)
}

/// Levenshtein distance with an early-exit cap, for did-you-mean (SPEC §3).
pub fn levenshtein_capped(a: &str, b: &str, cap: usize) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.len().abs_diff(b.len()) > cap {
        return cap + 1;
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for i in 1..=a.len() {
        cur[0] = i;
        let mut row_min = cur[0];
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
            row_min = row_min.min(cur[j]);
        }
        if row_min > cap {
            return cap + 1;
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ref_parse_elision_and_canonical_display() {
        let r = EntityRef::parse("character:孙悟空").unwrap();
        assert_eq!(r.ns, "default");
        assert_eq!(r.to_string(), "character:default/孙悟空");
        let r = EntityRef::parse("character:龙宫/敖广").unwrap();
        assert_eq!(r.to_string(), "character:龙宫/敖广");
    }

    #[test]
    fn ref_parse_rejects_bad_words() {
        assert!(EntityRef::parse("no-colon").is_err());
        assert!(EntityRef::parse("k:a b").is_err());
        assert!(EntityRef::parse("k:a/b/c").is_err());
        assert!(EntityRef::parse("k:").is_err());
    }

    #[test]
    fn ref_nfc_normalization() {
        // "é" composed vs decomposed must be the same ref.
        let composed = EntityRef::parse("k:caf\u{e9}").unwrap();
        let decomposed = EntityRef::parse("k:cafe\u{301}").unwrap();
        assert_eq!(composed, decomposed);
    }

    #[test]
    fn first_sentence_cjk_and_ascii() {
        assert_eq!(first_sentence("灵明石猴。东胜神洲出身。"), "灵明石猴。");
        assert_eq!(
            first_sentence("Runs v1.2 of the loop. More."),
            "Runs v1.2 of the loop."
        );
        assert_eq!(first_sentence("first line\nsecond"), "first line");
    }

    #[test]
    fn levenshtein_caps() {
        assert_eq!(levenshtein_capped("沈雨", "沈玉", 2), 1);
        assert!(levenshtein_capped("abc", "xyzabc", 2) > 2);
    }
}
