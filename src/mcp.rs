//! MCP server on stdio (SPEC §5): newline-delimited JSON-RPC, tools only.
//! Sixteen tools — seven queries, validate, drift, six write ops, schema_edit.
//! The vault is reloaded per call: a full rebuild is milliseconds, and always
//! correct beats cleverly cached.

use crate::graph::Graph;
use crate::model::RelEntry;
use crate::{drift, ops, query, schema_ops, validate, vault};
use anyhow::{Result, anyhow, bail};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

pub enum McpMode {
    /// One vault, fixed at startup (`--vault` / upward discovery).
    Single(PathBuf),
    /// Every vault found under a root (`--scan`); tools select via `vault`.
    Scan(PathBuf),
}

struct VaultEntry {
    name: String,
    dir: PathBuf,
    /// Present when schema.yaml exists but the vault cannot load at all.
    load_error: Option<String>,
}

/// Find vaults: every directory under `root` containing a schema.yaml
/// (gitignore-aware walk, hidden dirs skipped).
fn scan_vaults(root: &Path) -> Vec<VaultEntry> {
    let mut out = Vec::new();
    for entry in ignore::WalkBuilder::new(root)
        .hidden(true)
        .build()
        .flatten()
    {
        if entry.file_name() == "schema.yaml"
            && entry.file_type().is_some_and(|t| t.is_file())
            && let Some(dir) = entry.path().parent()
        {
            let (name, load_error) = match vault::load(dir) {
                Ok(v) => (v.schema.name.clone(), None),
                Err(e) => (
                    dir.file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    Some(format!("{e:#}")),
                ),
            };
            out.push(VaultEntry {
                name,
                dir: dir.to_path_buf(),
                load_error,
            });
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name).then(a.dir.cmp(&b.dir)));
    out
}

/// Resolve which vault a call targets. Single mode ignores the `vault` arg;
/// scan mode needs it whenever more than one loadable vault exists.
fn resolve_vault_dir(mode: &McpMode, args: &Value) -> Result<PathBuf> {
    match mode {
        McpMode::Single(dir) => Ok(dir.clone()),
        McpMode::Scan(root) => {
            let entries = scan_vaults(root);
            let loadable: Vec<&VaultEntry> =
                entries.iter().filter(|e| e.load_error.is_none()).collect();
            let wanted = args["vault"].as_str();
            match (wanted, loadable.as_slice()) {
                (Some(w), _) => loadable
                    .iter()
                    .find(|e| e.name == w || e.dir.ends_with(w))
                    .map(|e| e.dir.clone())
                    .ok_or_else(|| {
                        anyhow!(
                            "no vault `{w}` — available: {}",
                            loadable
                                .iter()
                                .map(|e| e.name.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    }),
                (None, [only]) => Ok(only.dir.clone()),
                (None, []) => bail!("no loadable vault under {}", root.display()),
                (None, many) => bail!(
                    "several vaults here — pass `vault`: {}",
                    many.iter()
                        .map(|e| e.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            }
        }
    }
}

pub fn serve(mode: McpMode) -> Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => {
                respond(
                    &mut stdout,
                    json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"parse error"}}),
                )?;
                continue;
            }
        };
        let id = msg.get("id").cloned();
        let method = msg["method"].as_str().unwrap_or("");
        // Notifications (no id) get no response.
        if id.is_none() {
            continue;
        }
        let reply = match method {
            "initialize" => json!({
                "protocolVersion": msg["params"]["protocolVersion"].as_str().unwrap_or("2024-11-05"),
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": "laplace", "version": env!("CARGO_PKG_VERSION") },
            }),
            "ping" => json!({}),
            "tools/list" => json!({ "tools": tool_defs() }),
            "tools/call" => {
                let name = msg["params"]["name"].as_str().unwrap_or("");
                let args = msg["params"]["arguments"].clone();
                match call(&mode, name, args) {
                    Ok(v) => json!({
                        "content": [{ "type": "text", "text": serde_json::to_string_pretty(&v)? }],
                        "isError": false,
                    }),
                    Err(e) => json!({
                        "content": [{ "type": "text", "text": format!("{e:#}") }],
                        "isError": true,
                    }),
                }
            }
            _ => {
                respond(
                    &mut stdout,
                    json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":format!("unknown method {method}")}}),
                )?;
                continue;
            }
        };
        respond(&mut stdout, json!({"jsonrpc":"2.0","id":id,"result":reply}))?;
    }
    Ok(())
}

fn respond(out: &mut impl Write, v: Value) -> Result<()> {
    writeln!(out, "{}", serde_json::to_string(&v)?)?;
    out.flush()?;
    Ok(())
}

fn call(mode: &McpMode, name: &str, args: Value) -> Result<Value> {
    if name == "laplace_vaults" {
        let root = match mode {
            McpMode::Single(dir) => dir.clone(),
            McpMode::Scan(root) => root.clone(),
        };
        let entries = match mode {
            McpMode::Single(dir) => vec![VaultEntry {
                name: vault::load(dir).map(|v| v.schema.name).unwrap_or_default(),
                dir: dir.clone(),
                load_error: None,
            }],
            McpMode::Scan(root) => scan_vaults(root),
        };
        return Ok(json!({
            "root": root,
            "vaults": entries.iter().map(|e| {
                match &e.load_error {
                    Some(err) => json!({ "name": e.name, "path": e.dir, "loadable": false, "error": err }),
                    None => match vault::load(&e.dir) {
                        Ok(v) => {
                            let r = validate::run(&v);
                            json!({
                                "name": e.name, "path": e.dir, "loadable": true,
                                "entities": v.entities.len(),
                                "errors": r.errors(), "warnings": r.warnings(),
                            })
                        }
                        Err(err) => json!({ "name": e.name, "path": e.dir, "loadable": false, "error": format!("{err:#}") }),
                    },
                }
            }).collect::<Vec<_>>(),
        }));
    }
    let vault_dir = resolve_vault_dir(mode, &args)?;
    let v = vault::load(&vault_dir)?;
    let s = |k: &str| -> Result<String> {
        args[k]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| anyhow!("missing required argument `{k}`"))
    };
    let opt = |k: &str| args[k].as_str().map(str::to_string);
    let list = |k: &str| -> Vec<String> {
        args[k]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    };

    // Queries refuse a broken truth; ops and validate/drift always run.
    let queries = [
        "laplace_search",
        "laplace_get",
        "laplace_neighbors",
        "laplace_trace",
        "laplace_impact",
        "laplace_architecture",
        "laplace_schema",
    ];
    if queries.contains(&name) {
        let report = validate::run(&v);
        if report.errors() > 0 {
            bail!(
                "refusing to query an invalid vault ({} errors) — run laplace_validate",
                report.errors()
            );
        }
    }

    match name {
        "laplace_search" => {
            let g = Graph::build(&v);
            Ok(query::search(
                &g,
                &s("q")?,
                opt("kind").as_deref(),
                opt("tag").as_deref(),
                args["limit"].as_u64().unwrap_or(20) as usize,
            ))
        }
        "laplace_get" => {
            let g = Graph::build(&v);
            Ok(query::get(&g, &v.resolve(&s("ref")?)?))
        }
        "laplace_neighbors" => {
            let g = Graph::build(&v);
            Ok(query::neighbors(
                &g,
                &v.resolve(&s("ref")?)?,
                (args["depth"].as_u64().unwrap_or(1) as usize).clamp(1, 2),
                &list("kinds"),
                &list("relations"),
            ))
        }
        "laplace_trace" => {
            let g = Graph::build(&v);
            Ok(query::trace(
                &g,
                &v.resolve(&s("from")?)?,
                &v.resolve(&s("to")?)?,
                args["limit"].as_u64().unwrap_or(5) as usize,
                args["max_len"].as_u64().unwrap_or(6) as usize,
            ))
        }
        "laplace_impact" => {
            let g = Graph::build(&v);
            Ok(query::impact(
                &g,
                &v.resolve(&s("ref")?)?,
                args["depth"].as_u64().unwrap_or(2) as usize,
                &list("via"),
            ))
        }
        "laplace_architecture" => Ok(query::architecture(&Graph::build(&v))),
        "laplace_schema" => Ok(query::schema(&Graph::build(&v))),
        "laplace_validate" => {
            let report = validate::run(&v);
            Ok(json!({
                "errors": report.errors(),
                "warnings": report.warnings(),
                "diagnostics": report.diags,
            }))
        }
        "laplace_drift" => drift::run(&v, opt("since").as_deref()),
        "laplace_add" => {
            let spec: ops::AddSpec =
                serde_json::from_value(args).map_err(|e| anyhow!("add spec: {e}"))?;
            ops::add(&v, spec).map(|o| o.json)
        }
        "laplace_update" => {
            let spec = ops::UpdateSpec {
                title: opt("title"),
                clear_title: args["clear_title"].as_bool().unwrap_or(false),
                lifecycle: opt("lifecycle"),
                clear_lifecycle: args["clear_lifecycle"].as_bool().unwrap_or(false),
                add_tags: list("add_tags"),
                remove_tags: list("remove_tags"),
                set: args["set"]
                    .as_object()
                    .map(|m| {
                        m.iter()
                            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                            .collect()
                    })
                    .unwrap_or_default(),
                unset: list("unset"),
                body: opt("body"),
            };
            ops::update(&v, &s("ref")?, spec).map(|o| o.json)
        }
        "laplace_link" => ops::link(&v, &s("from")?, &s("rel")?, &s("to")?, opt("note"))
            .map(|o| json!({ "result": o.message, "detail": o.json })),
        "laplace_unlink" => ops::unlink(&v, &s("from")?, &s("rel")?, &s("to")?).map(|o| o.json),
        "laplace_remove" => ops::remove(&v, &s("ref")?).map(|o| o.json),
        "laplace_rename" => {
            ops::rename(&v, &s("ref")?, &s("new_name")?, opt("namespace").as_deref())
                .map(|o| json!({ "result": o.message, "detail": o.json }))
        }
        "laplace_schema_edit" => {
            let op = s("op")?;
            match op.as_str() {
                "add-kind" => schema_ops::add_kind(&v, &s("name")?, opt("description")),
                "add-relation" => schema_ops::add_relation(
                    &v,
                    &s("name")?,
                    schema_ops::RelationSpec {
                        description: s("description").unwrap_or_default(),
                        propagation: opt("propagation")
                            .map(|p| serde_norway::from_str(&p))
                            .transpose()
                            .map_err(|_| anyhow!("propagation is to-source|to-target|both|none"))?,
                        symmetric: args["symmetric"].as_bool().unwrap_or(false),
                        acyclic: args["acyclic"].as_bool().unwrap_or(false),
                        from: args["from"].as_array().map(|_| list("from")),
                        to: args["to"].as_array().map(|_| list("to")),
                    },
                ),
                "set" => schema_ops::set(&v, &s("path")?, &s("value")?),
                "rename-kind" => schema_ops::rename_kind(&v, &s("old")?, &s("new")?),
                "rename-relation" => schema_ops::rename_relation(&v, &s("old")?, &s("new")?),
                other => bail!("unknown schema op `{other}`"),
            }
            .map(|o| json!({ "result": o.message, "detail": o.json }))
        }
        other => bail!("unknown tool `{other}`"),
    }
}

/// Relations for laplace_add arrive as {type: [target-or-object]} — same shape
/// as frontmatter, deserialized through the same RelEntry.
#[allow(dead_code)]
fn _rel_shape_doc(_: BTreeMap<String, Vec<RelEntry>>) {}

fn tool_defs() -> Vec<Value> {
    let obj = |props: Value, required: &[&str]| json!({ "type": "object", "properties": props, "required": required });
    let sp = |desc: &str| json!({ "type": "string", "description": desc });
    vec![
        json!({ "name": "laplace_search",
            "description": "Resolve names to refs — search before adding an entity or guessing a ref.",
            "inputSchema": obj(json!({ "q": sp("query text"), "kind": sp("filter by kind"), "tag": sp("filter by tag"), "limit": {"type":"integer"} }), &["q"]) }),
        json!({ "name": "laplace_get",
            "description": "One entity in full: frontmatter, prose body, edges in both directions, vault path.",
            "inputSchema": obj(json!({ "ref": sp("entity ref, e.g. character:孙悟空") }), &["ref"]) }),
        json!({ "name": "laplace_neighbors",
            "description": "The induced subgraph within 1–2 undirected hops — what surrounds this entity.",
            "inputSchema": obj(json!({ "ref": sp("center entity"), "depth": {"type":"integer","minimum":1,"maximum":2}, "kinds": {"type":"array","items":{"type":"string"}}, "relations": {"type":"array","items":{"type":"string"}} }), &["ref"]) }),
        json!({ "name": "laplace_trace",
            "description": "How are these two entities connected? Shortest annotated paths.",
            "inputSchema": obj(json!({ "from": sp("start ref"), "to": sp("end ref"), "limit": {"type":"integer"}, "max_len": {"type":"integer"} }), &["from","to"]) }),
        json!({ "name": "laplace_impact",
            "description": "What does changing this touch? Propagation closure, distance-bucketed — a candidate set to review, not an oracle; trust decays with distance.",
            "inputSchema": obj(json!({ "ref": sp("changed entity"), "depth": {"type":"integer"}, "via": {"type":"array","items":{"type":"string"},"description":"restrict to these relation types"} }), &["ref"]) }),
        json!({ "name": "laplace_architecture",
            "description": "Kind-level condensation of the whole map — the overview, and the usage precedent for vocabulary choices.",
            "inputSchema": obj(json!({}), &[]) }),
        json!({ "name": "laplace_schema",
            "description": "The constitution: charter, kinds with authoring guides, relation types with reading directions — the first stop before writing.",
            "inputSchema": obj(json!({}), &[]) }),
        json!({ "name": "laplace_validate",
            "description": "Validate the vault (structure, declarations, references, anchors) — run after any direct file edit.",
            "inputSchema": obj(json!({}), &[]) }),
        json!({ "name": "laplace_drift",
            "description": "Session-start freshness audit: stale entities, uncovered changed paths, unanchored ratio, dead anchors.",
            "inputSchema": obj(json!({ "since": sp("git rev to compare against (default: last commit touching the vault)") }), &[]) }),
        json!({ "name": "laplace_add",
            "description": "Create an entity — validated before anything touches disk. Search first; consult laplace_schema for the vocabulary.",
            "inputSchema": obj(json!({
                "kind": sp("declared kind"), "name": sp("entity name (any script)"), "namespace": sp("optional namespace"),
                "title": sp("display name"), "tags": {"type":"array","items":{"type":"string"}}, "lifecycle": sp("free string"),
                "body": sp("the prose description — what this is, why it exists"),
                "relations": {"type":"object","description":"{type: [\"kind:name\" or {ref, note}]}"},
                "source": {"type":"array","items":{"type":"string"},"description":"root-relative globs anchoring this entity"}
            }), &["kind","name"]) }),
        json!({ "name": "laplace_update",
            "description": "Set/unset fields of an entity; body only when explicitly given.",
            "inputSchema": obj(json!({ "ref": sp("entity"), "title": sp(""), "clear_title": {"type":"boolean"}, "lifecycle": sp(""), "clear_lifecycle": {"type":"boolean"}, "add_tags": {"type":"array","items":{"type":"string"}}, "remove_tags": {"type":"array","items":{"type":"string"}}, "set": {"type":"object"}, "unset": {"type":"array","items":{"type":"string"}}, "body": sp("replacement prose") }), &["ref"]) }),
        json!({ "name": "laplace_link",
            "description": "Add one relation edge. The result echoes the source's full edge list of that type — read it to catch slips.",
            "inputSchema": obj(json!({ "from": sp("source ref"), "rel": sp("declared relation type"), "to": sp("target ref"), "note": sp("optional edge note") }), &["from","rel","to"]) }),
        json!({ "name": "laplace_unlink",
            "description": "Remove one relation edge.",
            "inputSchema": obj(json!({ "from": sp("source ref"), "rel": sp("relation type"), "to": sp("target ref") }), &["from","rel","to"]) }),
        json!({ "name": "laplace_remove",
            "description": "Delete an entity — refuses while inbound refs exist, listing them.",
            "inputSchema": obj(json!({ "ref": sp("entity to delete") }), &["ref"]) }),
        json!({ "name": "laplace_rename",
            "description": "Rename/move an entity, atomically rewriting all inbound refs; prose mentions are reported, never rewritten.",
            "inputSchema": obj(json!({ "ref": sp("current ref"), "new_name": sp("new name"), "namespace": sp("optional new namespace") }), &["ref","new_name"]) }),
        json!({ "name": "laplace_schema_edit",
            "description": "Constitutional operations: add-kind / add-relation / set / rename-kind / rename-relation. Cite the charter question a vocabulary change serves.",
            "inputSchema": obj(json!({
                "op": { "type": "string", "enum": ["add-kind","add-relation","set","rename-kind","rename-relation"] },
                "name": sp("kind/relation name (add ops)"), "description": sp("reading-direction description (required for add-relation)"),
                "propagation": { "type": "string", "enum": ["to-source","to-target","both","none"] },
                "symmetric": {"type":"boolean"}, "acyclic": {"type":"boolean"},
                "from": {"type":"array","items":{"type":"string"}}, "to": {"type":"array","items":{"type":"string"}},
                "path": sp("(kinds|relations).<name>.<field> (set op)"), "value": sp("new value (set op)"),
                "old": sp("old name (rename ops)"), "new": sp("new name (rename ops)")
            }), &["op"]) }),
        json!({ "name": "laplace_vaults",
            "description": "List the vaults this server can see (name, path, entity count, validity) — the map of maps.",
            "inputSchema": { "type": "object", "properties": {}, "required": [] } }),
    ]
}
