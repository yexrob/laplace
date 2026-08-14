//! The seven query tools (SPEC §5). Every tool returns a serde_json::Value —
//! `--json` prints it verbatim; the text renderers live in main.rs.

use crate::graph::Graph;
use crate::model::EntityRef;
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap, VecDeque};

/// search: ranked refs. Score = max of matched tiers (SPEC §5).
pub fn search(g: &Graph, q: &str, kind: Option<&str>, tag: Option<&str>, limit: usize) -> Value {
    let ql = q.to_lowercase();
    let mut hits: Vec<(i32, &str, String, String, String)> = Vec::new();
    for e in &g.vault.entities {
        if let Some(k) = kind
            && e.eref.kind != k
        {
            continue;
        }
        if let Some(t) = tag
            && !e.fm.tags.iter().any(|x| x == t)
        {
            continue;
        }
        let name = e.eref.name.to_lowercase();
        let title = e.title().to_lowercase();
        let (score, field) = if name == ql {
            (100, "name-exact")
        } else if name.starts_with(&ql) {
            (80, "name-prefix")
        } else if name.contains(&ql) {
            (60, "name-substring")
        } else if title.contains(&ql) {
            (50, "title-substring")
        } else if e.fm.tags.iter().any(|t| t.to_lowercase() == ql) {
            (40, "tag-exact")
        } else if e.body.to_lowercase().contains(&ql) {
            (20, "body-substring")
        } else {
            continue;
        };
        hits.push((
            score,
            field,
            e.eref.kind.clone(),
            e.eref.to_string(),
            e.first_sentence(),
        ));
    }
    hits.sort_by(|a, b| b.0.cmp(&a.0).then(a.2.cmp(&b.2)).then(a.3.cmp(&b.3)));
    hits.truncate(limit);
    json!({
        "query": q,
        "results": hits.iter().map(|(score, field, _, r, fs)| json!({
            "ref": r, "score": score, "matched": field, "summary": fs,
        })).collect::<Vec<_>>(),
    })
}

/// get: the full document + computed edges both directions + vault path.
pub fn get(g: &Graph, eref: &EntityRef) -> Value {
    let idx = g.index[eref];
    let e = g.entity(idx);
    let mut outbound: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for &ei in g.out.get(&idx).into_iter().flatten() {
        let edge = &g.edges[ei];
        outbound
            .entry(edge.rel.clone())
            .or_default()
            .push(edge_json(g, edge, edge.target));
    }
    let mut inbound: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for &ei in g.inc.get(&idx).into_iter().flatten() {
        let edge = &g.edges[ei];
        inbound
            .entry(edge.rel.clone())
            .or_default()
            .push(edge_json(g, edge, edge.source));
    }
    json!({
        "ref": eref.to_string(),
        "path": g.vault.dir.join(&e.file).display().to_string(),
        "title": e.title(),
        "tags": e.fm.tags,
        "lifecycle": e.fm.lifecycle,
        "extra": e.fm.extra,
        "source": e.fm.source,
        "body": e.body.trim(),
        "outbound": outbound,
        "inbound": inbound,
    })
}

fn edge_json(g: &Graph, edge: &crate::graph::Edge, other: usize) -> Value {
    let mut v = json!({ "ref": g.entity(other).eref.to_string() });
    if let Some(n) = &edge.note {
        v["note"] = json!(n);
    }
    v
}

/// neighbors: induced subgraph within `depth` (≤2) undirected hops.
pub fn neighbors(
    g: &Graph,
    eref: &EntityRef,
    depth: usize,
    kinds: &[String],
    relations: &[String],
) -> Value {
    let start = g.index[eref];
    let mut dist: HashMap<usize, usize> = HashMap::from([(start, 0)]);
    let mut queue = VecDeque::from([start]);
    while let Some(n) = queue.pop_front() {
        let d = dist[&n];
        if d == depth {
            continue;
        }
        for step in g.undirected(n) {
            if !relations.is_empty() && !relations.contains(&g.edges[step.edge].rel) {
                continue;
            }
            let other_kind = &g.entity(step.other).eref.kind;
            if !kinds.is_empty() && step.other != start && !kinds.contains(other_kind) {
                continue;
            }
            if let std::collections::hash_map::Entry::Vacant(e) = dist.entry(step.other) {
                e.insert(d + 1);
                queue.push_back(step.other);
            }
        }
    }
    let mut nodes: Vec<_> = dist.iter().collect();
    nodes.sort_by_key(|(idx, d)| (**d, g.entity(**idx).eref.to_string()));
    let edges: Vec<Value> = g
        .edges
        .iter()
        .filter(|e| dist.contains_key(&e.source) && dist.contains_key(&e.target))
        .filter(|e| relations.is_empty() || relations.contains(&e.rel))
        .map(|e| {
            let mut v = json!({
                "from": g.entity(e.source).eref.to_string(),
                "rel": e.rel,
                "to": g.entity(e.target).eref.to_string(),
            });
            if g.is_symmetric(&e.rel) {
                v["symmetric"] = json!(true);
            }
            if let Some(n) = &e.note {
                v["note"] = json!(n);
            }
            v
        })
        .collect();
    json!({
        "center": eref.to_string(),
        "depth": depth,
        "nodes": nodes.iter().map(|(idx, d)| {
            let e = g.entity(**idx);
            json!({ "ref": e.eref.to_string(), "distance": d, "title": e.title(), "summary": e.first_sentence() })
        }).collect::<Vec<_>>(),
        "edges": edges,
    })
}

/// trace: up to `limit` shortest simple paths (undirected), each hop annotated.
pub fn trace(g: &Graph, from: &EntityRef, to: &EntityRef, limit: usize, max_len: usize) -> Value {
    let (s, t) = (g.index[from], g.index[to]);
    // BFS distances from the target for pruning.
    let mut dist_to_t: HashMap<usize, usize> = HashMap::from([(t, 0)]);
    let mut q = VecDeque::from([t]);
    while let Some(n) = q.pop_front() {
        for step in g.undirected(n) {
            if !dist_to_t.contains_key(&step.other) {
                dist_to_t.insert(step.other, dist_to_t[&n] + 1);
                q.push_back(step.other);
            }
        }
    }
    let mut paths: Vec<Vec<(usize, bool)>> = Vec::new(); // (edge, forward) hops
    let mut budget = 20_000usize; // exploration safeguard
    let mut stack_path: Vec<(usize, bool)> = Vec::new();
    let mut on_path = vec![false; g.vault.entities.len()];
    #[allow(clippy::too_many_arguments)]
    fn dfs(
        g: &Graph,
        node: usize,
        t: usize,
        max_len: usize,
        dist_to_t: &HashMap<usize, usize>,
        stack_path: &mut Vec<(usize, bool)>,
        on_path: &mut [bool],
        paths: &mut Vec<Vec<(usize, bool)>>,
        budget: &mut usize,
    ) {
        if *budget == 0 {
            return;
        }
        *budget -= 1;
        if node == t {
            paths.push(stack_path.clone());
            return;
        }
        let Some(&d) = dist_to_t.get(&node) else {
            return;
        };
        if stack_path.len() + d > max_len {
            return;
        }
        on_path[node] = true;
        for step in g.undirected(node) {
            if on_path[step.other] {
                continue;
            }
            stack_path.push((step.edge, step.forward));
            dfs(
                g, step.other, t, max_len, dist_to_t, stack_path, on_path, paths, budget,
            );
            stack_path.pop();
        }
        on_path[node] = false;
    }
    if dist_to_t.contains_key(&s) {
        dfs(
            g,
            s,
            t,
            max_len,
            &dist_to_t,
            &mut stack_path,
            &mut on_path,
            &mut paths,
            &mut budget,
        );
    }
    paths.sort_by_key(|p| p.len());
    paths.truncate(limit);
    json!({
        "from": from.to_string(),
        "to": to.to_string(),
        "paths": paths.iter().map(|p| {
            let mut node = s;
            let hops: Vec<Value> = p.iter().map(|&(ei, fwd)| {
                let e = &g.edges[ei];
                let (dir, next) = if g.is_symmetric(&e.rel) {
                    ("<->", if fwd { e.target } else { e.source })
                } else if fwd {
                    ("->", e.target)
                } else {
                    ("<-", e.source)
                };
                node = next;
                json!({ "rel": e.rel, "direction": dir, "to": g.entity(next).eref.to_string() })
            }).collect();
            json!({ "length": p.len(), "hops": hops })
        }).collect::<Vec<_>>(),
    })
}

/// impact: BFS closure over the propagation digraph, distance-bucketed, one
/// shortest witness path per entity. A candidate set, not an oracle.
pub fn impact(g: &Graph, eref: &EntityRef, depth: usize, via: &[String]) -> Value {
    let start = g.index[eref];
    let mut dist: HashMap<usize, usize> = HashMap::from([(start, 0)]);
    let mut parent: HashMap<usize, (usize, usize)> = HashMap::new(); // node → (prev node, edge)
    let mut queue = VecDeque::from([start]);
    while let Some(n) = queue.pop_front() {
        let d = dist[&n];
        if d == depth {
            continue;
        }
        for step in g.propagation(n) {
            if !via.is_empty() && !via.contains(&g.edges[step.edge].rel) {
                continue;
            }
            if let std::collections::hash_map::Entry::Vacant(e) = dist.entry(step.other) {
                e.insert(d + 1);
                parent.insert(step.other, (n, step.edge));
                queue.push_back(step.other);
            }
        }
    }
    let mut buckets: BTreeMap<usize, Vec<Value>> = BTreeMap::new();
    let mut affected: Vec<_> = dist.iter().filter(|(i, _)| **i != start).collect();
    affected.sort_by_key(|(i, d)| (**d, g.entity(**i).eref.to_string()));
    for (&idx, &d) in affected {
        // Witness path: walk parents back to start, then reverse.
        let mut hops = Vec::new();
        let mut cur = idx;
        while cur != start {
            let (prev, ei) = parent[&cur];
            let e = &g.edges[ei];
            hops.push(json!({
                "via": e.rel,
                "from": g.entity(prev).eref.to_string(),
                "to": g.entity(cur).eref.to_string(),
            }));
            cur = prev;
        }
        hops.reverse();
        buckets.entry(d).or_default().push(json!({
            "ref": g.entity(idx).eref.to_string(),
            "summary": g.entity(idx).first_sentence(),
            "path": hops,
        }));
    }
    json!({
        "changed": eref.to_string(),
        "depth": depth,
        "affected": dist.len() - 1,
        "buckets": buckets.into_iter().map(|(d, items)| json!({
            "distance": d, "entities": items,
        })).collect::<Vec<_>>(),
    })
}

/// architecture: kind-level condensation — the overview that IS safe to render.
pub fn architecture(g: &Graph) -> Value {
    let mut kind_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for e in &g.vault.entities {
        *kind_counts.entry(e.eref.kind.as_str()).or_default() += 1;
    }
    let mut agg: BTreeMap<(String, String, String), usize> = BTreeMap::new();
    for e in &g.edges {
        let key = (
            g.entity(e.source).eref.kind.clone(),
            e.rel.clone(),
            g.entity(e.target).eref.kind.clone(),
        );
        *agg.entry(key).or_default() += 1;
    }
    json!({
        "project": g.vault.schema.title.clone().unwrap_or(g.vault.schema.name.clone()),
        "entities": g.vault.entities.len(),
        "relations": g.edges.len(),
        "kinds": kind_counts.iter().map(|(k, c)| json!({ "kind": k, "count": c })).collect::<Vec<_>>(),
        "edges": agg.iter().map(|((f, r, t), c)| json!({
            "from_kind": f, "rel": r, "to_kind": t, "count": c,
        })).collect::<Vec<_>>(),
    })
}

/// schema: the constitution — the agent's first stop before writing.
pub fn schema(g: &Graph) -> Value {
    let s = &g.vault.schema;
    json!({
        "apiVersion": s.api_version,
        "name": s.name,
        "title": s.title,
        "root": s.root,
        "charter": s.charter,
        "ignore": s.ignore,
        "exclusions": s.exclusions,
        "kinds": s.kinds.iter().map(|(k, d)| json!({
            "kind": k,
            "label": d.label(),
            "description": d.description,
        })).collect::<Vec<_>>(),
        "relations": s.relations.iter().map(|(r, d)| {
            let mut v = json!({
                "relation": r,
                "description": d.description,
                "propagation": d.propagation.as_str(),
            });
            if d.symmetric { v["symmetric"] = json!(true); }
            if d.acyclic { v["acyclic"] = json!(true); }
            if let Some(f) = &d.from { v["from"] = json!(f); }
            if let Some(t) = &d.to { v["to"] = json!(t); }
            v
        }).collect::<Vec<_>>(),
    })
}
