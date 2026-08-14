//! The in-memory property graph — a derived cache of the vault (SPEC §4).
//! Built over a validated vault: unresolvable targets are simply skipped here,
//! because queries refuse to run when validate reports errors.

use crate::model::{EntityRef, Propagation};
use crate::vault::Vault;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug)]
pub struct Edge {
    pub source: usize,
    pub rel: String,
    pub target: usize,
    pub note: Option<String>,
}

pub struct Graph<'a> {
    pub vault: &'a Vault,
    pub index: BTreeMap<EntityRef, usize>,
    pub edges: Vec<Edge>,
    pub out: HashMap<usize, Vec<usize>>,
    pub inc: HashMap<usize, Vec<usize>>,
}

/// A traversal step over one edge, seen from a given node.
#[derive(Clone, Copy)]
pub struct Step {
    pub edge: usize,
    pub other: usize,
    /// true when traversed source→target (the declared direction).
    pub forward: bool,
}

impl<'a> Graph<'a> {
    pub fn build(vault: &'a Vault) -> Self {
        let index: BTreeMap<EntityRef, usize> = vault
            .entities
            .iter()
            .enumerate()
            .map(|(i, e)| (e.eref.clone(), i))
            .collect();
        let mut edges = Vec::new();
        let mut out: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut inc: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut seen = std::collections::HashSet::new();
        for (si, e) in vault.entities.iter().enumerate() {
            for (rel, entries) in &e.fm.relations {
                let Some(decl) = vault.schema.relations.get(rel) else {
                    continue;
                };
                for entry in entries {
                    let Ok(target) = EntityRef::parse(entry.target()) else {
                        continue;
                    };
                    let Some(&ti) = index.get(&target) else {
                        continue;
                    };
                    // Dedup exact duplicates and both-sides symmetric declarations.
                    let key = if decl.symmetric {
                        let (a, b) = (si.min(ti), si.max(ti));
                        (a, rel.clone(), b)
                    } else {
                        (si, rel.clone(), ti)
                    };
                    if !seen.insert(key) {
                        continue;
                    }
                    let ei = edges.len();
                    edges.push(Edge {
                        source: si,
                        rel: rel.clone(),
                        target: ti,
                        note: entry.note().map(str::to_string),
                    });
                    out.entry(si).or_default().push(ei);
                    inc.entry(ti).or_default().push(ei);
                }
            }
        }
        Self {
            vault,
            index,
            edges,
            out,
            inc,
        }
    }

    pub fn entity(&self, idx: usize) -> &crate::model::Entity {
        &self.vault.entities[idx]
    }

    pub fn is_symmetric(&self, rel: &str) -> bool {
        self.vault
            .schema
            .relations
            .get(rel)
            .is_some_and(|d| d.symmetric)
    }

    /// Undirected traversal steps from a node (plain graph: trace/neighbors).
    pub fn undirected(&self, idx: usize) -> Vec<Step> {
        let mut steps = Vec::new();
        for &ei in self.out.get(&idx).into_iter().flatten() {
            steps.push(Step {
                edge: ei,
                other: self.edges[ei].target,
                forward: true,
            });
        }
        for &ei in self.inc.get(&idx).into_iter().flatten() {
            steps.push(Step {
                edge: ei,
                other: self.edges[ei].source,
                forward: false,
            });
        }
        steps
    }

    /// Propagation-digraph steps from a node (impact): where does a change AT
    /// `idx` travel next? For an edge A→B, `to-source` means a change to B
    /// affects A (step runs target→source); `to-target` runs source→target.
    /// Symmetric relations carry `both` or `none`, so direction never matters.
    pub fn propagation(&self, idx: usize) -> Vec<Step> {
        self.undirected(idx)
            .into_iter()
            .filter(|step| {
                let decl = &self.vault.schema.relations[&self.edges[step.edge].rel];
                match decl.propagation {
                    Propagation::Both => true,
                    Propagation::None => false,
                    // We are at the target end iff !forward; the change travels to the source.
                    Propagation::ToSource => !step.forward,
                    Propagation::ToTarget => step.forward,
                }
            })
            .collect()
    }
}
