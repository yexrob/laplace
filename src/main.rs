//! laplace — the intellect that knows every entity in your project and how they connect.

use anyhow::Result;
use clap::{Parser, Subcommand};
use laplace::graph::Graph;
use laplace::{drift, mcp, ops, query, schema_ops, serve, skill, summary, validate, vault};
use serde_json::Value;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "laplace",
    version,
    about = "Entity map: single truth vault, graph queries, projections"
)]
struct Cli {
    /// Vault directory (contains schema.yaml). Default: search upward for laplace/schema.yaml.
    #[arg(long, global = true)]
    vault: Option<PathBuf>,
    /// Machine-readable JSON output.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Scaffold ./laplace/schema.yaml — only if absent.
    Init {
        /// Project name (default: the current directory's name).
        #[arg(long)]
        name: Option<String>,
    },
    /// Validate the vault: structure, declarations, references, anchors.
    Validate,
    /// Query the graph.
    #[command(subcommand)]
    Query(QueryCmd),
    /// Create an entity — validated before anything touches disk.
    Add {
        kind: Option<String>,
        name: Option<String>,
        #[arg(long)]
        ns: Option<String>,
        #[arg(long)]
        title: Option<String>,
        #[arg(long = "tag")]
        tags: Vec<String>,
        #[arg(long)]
        lifecycle: Option<String>,
        /// The prose description — what this is, why it exists.
        #[arg(long)]
        body: Option<String>,
        /// Repeatable: "type=target", e.g. --rel "持有=artifact:如意金箍棒".
        #[arg(long = "rel")]
        rels: Vec<String>,
        /// Repeatable: root-relative glob anchoring this entity.
        #[arg(long = "source")]
        sources: Vec<String>,
        /// Read a full AddSpec as JSON from stdin instead of flags.
        #[arg(long)]
        stdin: bool,
    },
    /// Set/unset fields of an entity; body only when explicitly given.
    Update {
        r#ref: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        clear_title: bool,
        #[arg(long)]
        lifecycle: Option<String>,
        #[arg(long)]
        clear_lifecycle: bool,
        #[arg(long = "tag")]
        add_tags: Vec<String>,
        #[arg(long = "untag")]
        remove_tags: Vec<String>,
        /// Repeatable free-form key: --set k=v.
        #[arg(long = "set")]
        set: Vec<String>,
        #[arg(long = "unset")]
        unset: Vec<String>,
        #[arg(long)]
        body: Option<String>,
    },
    /// Add one relation edge; echoes the full edge list of that type.
    Link {
        from: String,
        rel: String,
        to: String,
        #[arg(long)]
        note: Option<String>,
    },
    /// Remove one relation edge.
    Unlink {
        from: String,
        rel: String,
        to: String,
    },
    /// Delete an entity — refuses while inbound refs exist.
    Remove { r#ref: String },
    /// Rename/move an entity, rewriting all inbound refs atomically.
    Rename {
        r#ref: String,
        new_name: String,
        #[arg(long)]
        ns: Option<String>,
    },
    /// Constitutional operations on schema.yaml.
    #[command(subcommand)]
    Schema(SchemaCmd),
    /// Session-start freshness audit against git history.
    Drift {
        #[arg(long)]
        since: Option<String>,
    },
    /// Full graph JSON to stdout — the jq/pipeline escape hatch.
    Export,
    /// The context-injection block: entity index + relation digest + recent
    /// changes, tiered to a token budget (CJK-aware estimation).
    Summary {
        #[arg(long, default_value_t = laplace::summary::DEFAULT_BUDGET)]
        budget: usize,
    },
    /// The entity-map skill: print it, or install it into harness skill dirs.
    #[command(subcommand)]
    Skill(SkillCmd),
    /// Read-only HTML view: overview plate, kind registers, entry pages.
    Serve {
        #[arg(long, default_value_t = 6174)]
        port: u16,
    },
    /// MCP server on stdio (17 tools).
    Mcp {
        /// Scan DIR (default ".") for every vault instead of serving one;
        /// tools then take a `vault` selector, `laplace_vaults` lists them.
        #[arg(long, num_args = 0..=1, default_missing_value = ".")]
        scan: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum SkillCmd {
    /// Print the full skill text to stdout.
    Show,
    /// Install <dir>/entity-map/SKILL.md into detected harness skill
    /// directories (~/.claude, ~/.config/bingo, ./.claude, ./.bingo), or --to DIR.
    Install {
        #[arg(long)]
        to: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum SchemaCmd {
    /// Declare a kind.
    AddKind {
        name: String,
        #[arg(long)]
        description: Option<String>,
    },
    /// Declare a relation type — the description must state the reading direction.
    AddRelation {
        name: String,
        #[arg(long)]
        description: String,
        #[arg(long)]
        propagation: Option<String>,
        #[arg(long)]
        symmetric: bool,
        #[arg(long)]
        acyclic: bool,
        #[arg(long, value_delimiter = ',')]
        from: Option<Vec<String>>,
        #[arg(long, value_delimiter = ',')]
        to: Option<Vec<String>>,
    },
    /// Set one declaration field: (kinds|relations).<name>.<field> <value>.
    Set { path: String, value: String },
    /// Rename a kind: moves the directory and rewrites every ref vault-wide.
    RenameKind { old: String, new: String },
    /// Rename a relation type: rewrites every usage vault-wide.
    RenameRelation { old: String, new: String },
}

#[derive(Subcommand)]
enum QueryCmd {
    /// Resolve names to refs — search before adding or guessing.
    Search {
        q: String,
        #[arg(long)]
        kind: Option<String>,
        #[arg(long)]
        tag: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// One entity in full: frontmatter, body, edges both directions, path.
    Get { r#ref: String },
    /// The induced subgraph around an entity (1–2 undirected hops).
    Neighbors {
        r#ref: String,
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..=2))]
        depth: u8,
        #[arg(long, value_delimiter = ',')]
        kinds: Vec<String>,
        #[arg(long, value_delimiter = ',')]
        relations: Vec<String>,
    },
    /// How are these two connected? Shortest simple paths, hop-annotated.
    Trace {
        from: String,
        to: String,
        #[arg(long, default_value_t = 5)]
        limit: usize,
        #[arg(long, default_value_t = 6)]
        max_len: usize,
    },
    /// What does changing this touch? Propagation closure, distance-bucketed.
    Impact {
        r#ref: String,
        #[arg(long, default_value_t = 2)]
        depth: usize,
        #[arg(long, value_delimiter = ',')]
        via: Vec<String>,
    },
    /// Kind-level condensation: the whole-map overview.
    Architecture,
    /// The constitution: charter, kinds, relation types with directions.
    Schema,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("laplace: {e:#}");
            ExitCode::from(2)
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    let cwd = std::env::current_dir()?;
    if let Cmd::Init { name } = &cli.cmd {
        return init(&cwd, name.as_deref());
    }
    if let Cmd::Skill(sc) = &cli.cmd {
        match sc {
            SkillCmd::Show => print!("{}", skill::SKILL_TEXT),
            SkillCmd::Install { to } => {
                for path in skill::install(&cwd, to.as_deref())? {
                    println!("installed {}", path.display());
                }
            }
        }
        return Ok(ExitCode::SUCCESS);
    }
    if let Cmd::Mcp { scan } = &cli.cmd {
        let mode = match scan {
            Some(root) => mcp::McpMode::Scan(root.clone()),
            None => match vault::discover(&cwd, cli.vault.as_deref()) {
                Ok(dir) => mcp::McpMode::Single(dir),
                Err(_) if cli.vault.is_none() => mcp::McpMode::Empty(cwd.clone()),
                Err(e) => return Err(e),
            },
        };
        mcp::serve(mode)?;
        return Ok(ExitCode::SUCCESS);
    }
    let dir = vault::discover(&cwd, cli.vault.as_deref())?;
    if let Cmd::Serve { port } = &cli.cmd {
        serve::serve(dir, *port)?;
        return Ok(ExitCode::SUCCESS);
    }
    let vault = vault::load(&dir)?;

    // The write operations and drift/export.
    let op_result: Option<Result<ops::Outcome>> = match &cli.cmd {
        Cmd::Add {
            kind,
            name,
            ns,
            title,
            tags,
            lifecycle,
            body,
            rels,
            sources,
            stdin,
        } => Some(
            build_add_spec(
                kind, name, ns, title, tags, lifecycle, body, rels, sources, *stdin,
            )
            .and_then(|spec| ops::add(&vault, spec)),
        ),
        Cmd::Update {
            r#ref,
            title,
            clear_title,
            lifecycle,
            clear_lifecycle,
            add_tags,
            remove_tags,
            set,
            unset,
            body,
        } => {
            let parsed_set: Result<Vec<(String, String)>> = set
                .iter()
                .map(|kv| {
                    kv.split_once('=')
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .ok_or_else(|| anyhow::anyhow!("--set takes k=v, got `{kv}`"))
                })
                .collect();
            Some(parsed_set.and_then(|set| {
                ops::update(
                    &vault,
                    r#ref,
                    ops::UpdateSpec {
                        title: title.clone(),
                        clear_title: *clear_title,
                        lifecycle: lifecycle.clone(),
                        clear_lifecycle: *clear_lifecycle,
                        add_tags: add_tags.clone(),
                        remove_tags: remove_tags.clone(),
                        set,
                        unset: unset.clone(),
                        body: body.clone(),
                    },
                )
            }))
        }
        Cmd::Link {
            from,
            rel,
            to,
            note,
        } => Some(ops::link(&vault, from, rel, to, note.clone())),
        Cmd::Unlink { from, rel, to } => Some(ops::unlink(&vault, from, rel, to)),
        Cmd::Remove { r#ref } => Some(ops::remove(&vault, r#ref)),
        Cmd::Rename {
            r#ref,
            new_name,
            ns,
        } => Some(ops::rename(&vault, r#ref, new_name, ns.as_deref())),
        Cmd::Schema(sc) => Some(match sc {
            SchemaCmd::AddKind { name, description } => {
                schema_ops::add_kind(&vault, name, description.clone())
            }
            SchemaCmd::AddRelation {
                name,
                description,
                propagation,
                symmetric,
                acyclic,
                from,
                to,
            } => {
                let prop = propagation
                    .as_deref()
                    .map(serde_norway::from_str)
                    .transpose()
                    .map_err(|_| anyhow::anyhow!("propagation is to-source|to-target|both|none"));
                prop.and_then(|propagation| {
                    schema_ops::add_relation(
                        &vault,
                        name,
                        schema_ops::RelationSpec {
                            description: description.clone(),
                            propagation,
                            symmetric: *symmetric,
                            acyclic: *acyclic,
                            from: from.clone(),
                            to: to.clone(),
                        },
                    )
                })
            }
            SchemaCmd::Set { path, value } => schema_ops::set(&vault, path, value),
            SchemaCmd::RenameKind { old, new } => schema_ops::rename_kind(&vault, old, new),
            SchemaCmd::RenameRelation { old, new } => schema_ops::rename_relation(&vault, old, new),
        }),
        _ => None,
    };
    if let Some(result) = op_result {
        return Ok(match result {
            Ok(outcome) => {
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&outcome.json)?);
                } else {
                    println!("{}", outcome.message);
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("laplace: {e:#}");
                ExitCode::from(1)
            }
        });
    }
    if let Cmd::Summary { budget } = &cli.cmd {
        let s = summary::render(&vault, *budget);
        if cli.json {
            println!("{}", serde_json::to_string_pretty(&summary::to_json(&s))?);
        } else {
            println!("{}", s.text);
        }
        return Ok(ExitCode::SUCCESS);
    }
    if let Cmd::Drift { since } = &cli.cmd {
        let v = drift::run(&vault, since.as_deref())?;
        if cli.json {
            println!("{}", serde_json::to_string_pretty(&v)?);
        } else {
            print!("{}", drift::render_text(&v));
        }
        return Ok(ExitCode::SUCCESS);
    }

    let report = validate::run(&vault);

    match cli.cmd {
        Cmd::Export => {
            if report.errors() > 0 {
                eprintln!(
                    "refusing to export an invalid vault ({} errors)",
                    report.errors()
                );
                return Ok(ExitCode::from(1));
            }
            let g = Graph::build(&vault);
            println!("{}", serde_json::to_string_pretty(&query::export(&g))?);
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Validate => {
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&report.diags)?);
            } else {
                for d in &report.diags {
                    println!("{}\n", d.render());
                }
                println!(
                    "{}: {} entities · {} errors · {} warnings",
                    vault.schema.name,
                    vault.entities.len(),
                    report.errors(),
                    report.warnings()
                );
            }
            Ok(if report.errors() > 0 {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            })
        }
        Cmd::Query(q) => {
            // No queries over a broken truth (SPEC §4).
            if report.errors() > 0 {
                for d in report
                    .diags
                    .iter()
                    .filter(|d| d.severity == validate::Severity::Error)
                {
                    eprintln!("{}\n", d.render());
                }
                eprintln!(
                    "refusing to query an invalid vault ({} errors)",
                    report.errors()
                );
                return Ok(ExitCode::from(1));
            }
            let g = Graph::build(&vault);
            let value = match &q {
                QueryCmd::Search {
                    q,
                    kind,
                    tag,
                    limit,
                } => query::search(&g, q, kind.as_deref(), tag.as_deref(), *limit),
                QueryCmd::Get { r#ref } => query::get(&g, &vault.resolve(r#ref)?),
                QueryCmd::Neighbors {
                    r#ref,
                    depth,
                    kinds,
                    relations,
                } => query::neighbors(
                    &g,
                    &vault.resolve(r#ref)?,
                    *depth as usize,
                    kinds,
                    relations,
                ),
                QueryCmd::Trace {
                    from,
                    to,
                    limit,
                    max_len,
                } => query::trace(
                    &g,
                    &vault.resolve(from)?,
                    &vault.resolve(to)?,
                    *limit,
                    *max_len,
                ),
                QueryCmd::Impact { r#ref, depth, via } => {
                    query::impact(&g, &vault.resolve(r#ref)?, *depth, via)
                }
                QueryCmd::Architecture => query::architecture(&g),
                QueryCmd::Schema => query::schema(&g),
            };
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&value)?);
            } else {
                render_text(&q, &value);
            }
            Ok(ExitCode::SUCCESS)
        }
        _ => unreachable!("handled above"),
    }
}

#[allow(clippy::too_many_arguments)]
fn build_add_spec(
    kind: &Option<String>,
    name: &Option<String>,
    ns: &Option<String>,
    title: &Option<String>,
    tags: &[String],
    lifecycle: &Option<String>,
    body: &Option<String>,
    rels: &[String],
    sources: &[String],
    stdin: bool,
) -> Result<ops::AddSpec> {
    if stdin {
        let mut buf = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)?;
        return Ok(serde_json::from_str(&buf)?);
    }
    let (Some(kind), Some(name)) = (kind, name) else {
        anyhow::bail!("laplace add <kind> <name> … (or --stdin with a JSON spec)");
    };
    let mut relations: std::collections::BTreeMap<String, Vec<laplace::model::RelEntry>> =
        Default::default();
    for r in rels {
        let (t, target) = r
            .split_once('=')
            .ok_or_else(|| anyhow::anyhow!("--rel takes type=target, got `{r}`"))?;
        relations
            .entry(t.to_string())
            .or_default()
            .push(laplace::model::RelEntry::Bare(target.to_string()));
    }
    Ok(ops::AddSpec {
        kind: kind.clone(),
        name: name.clone(),
        namespace: ns.clone(),
        title: title.clone(),
        tags: tags.to_vec(),
        lifecycle: lifecycle.clone(),
        body: body.clone().unwrap_or_default(),
        relations,
        source: sources.to_vec(),
    })
}

fn init(cwd: &std::path::Path, name: Option<&str>) -> Result<ExitCode> {
    let dir = cwd.join("laplace");
    let schema = dir.join("schema.yaml");
    if schema.exists() {
        anyhow::bail!("{} already exists — nothing scaffolded", schema.display());
    }
    let project = name
        .map(str::to_string)
        .or_else(|| cwd.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "project".into());
    std::fs::create_dir_all(&dir)?;
    std::fs::write(
        &schema,
        format!(
            r#"apiVersion: laplace/v1
name: {project}
# root: ..            # what source/ignore globs resolve against (default: the directory containing this vault)

# The charter: the questions this map exists to answer. Derive the vocabulary
# below from these questions — not from ontological completeness.
charter:
  - 改动 X，哪些东西必须跟着重看？

# ignore: []          # declared non-territory: paths no entity will ever claim
# exclusions: []      # concept-shaped non-goals, with reasons

# Kinds: the nouns. A description's first sentence is its label; the rest is
# the authoring guide surfaced at `laplace add` time.
kinds:
  thing: {{ description: 一类东西。描述应写清它为什么存在、负责什么。 }}

# Relations: the verbs. Every relation MUST state its reading direction.
# propagation — the two-question test for `A rel B`:
#   B changed, must A be revisited?  yes → to-source
#   A changed, must B be revisited?  yes → to-target   (both/none accordingly)
relations:
  depends-on:
    description: A depends-on B —— A 是消费方，B 是被依赖方。改 B 要回头看 A。
    propagation: to-source
"#
        ),
    )?;
    println!(
        "scaffolded {} — edit the constitution, then `laplace add` your first entity",
        schema.display()
    );
    Ok(ExitCode::SUCCESS)
}

fn render_text(cmd: &QueryCmd, v: &Value) {
    match cmd {
        QueryCmd::Search { .. } => {
            let results = v["results"].as_array().unwrap();
            if results.is_empty() {
                println!("no matches");
                return;
            }
            for r in results {
                println!(
                    "{:<40} {:<4} {:<15} {}",
                    r["ref"].as_str().unwrap(),
                    r["score"],
                    r["matched"].as_str().unwrap(),
                    r["summary"].as_str().unwrap_or("")
                );
            }
        }
        QueryCmd::Get { .. } => {
            println!(
                "{}  ({})",
                v["ref"].as_str().unwrap(),
                v["path"].as_str().unwrap()
            );
            if let Some(t) = v["title"].as_str() {
                println!("title: {t}");
            }
            let tags = v["tags"].as_array().unwrap();
            if !tags.is_empty() {
                println!(
                    "tags: {}",
                    tags.iter()
                        .filter_map(|t| t.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
            if let Some(l) = v["lifecycle"].as_str() {
                println!("lifecycle: {l}");
            }
            for (label, key) in [("outbound", "outbound"), ("inbound", "inbound")] {
                let map = v[key].as_object().unwrap();
                if !map.is_empty() {
                    println!("{label}:");
                    for (rel, targets) in map {
                        for t in targets.as_array().unwrap() {
                            let arrow = if key == "outbound" { "→" } else { "←" };
                            let note = t["note"]
                                .as_str()
                                .map(|n| format!("  ({n})"))
                                .unwrap_or_default();
                            println!("  {arrow} {rel} {}{note}", t["ref"].as_str().unwrap());
                        }
                    }
                }
            }
            println!("\n{}", v["body"].as_str().unwrap());
        }
        QueryCmd::Neighbors { .. } => {
            println!(
                "neighborhood of {} (depth {})",
                v["center"].as_str().unwrap(),
                v["depth"]
            );
            for n in v["nodes"].as_array().unwrap() {
                println!(
                    "  [{}] {:<40} {}",
                    n["distance"],
                    n["ref"].as_str().unwrap(),
                    n["summary"].as_str().unwrap_or("")
                );
            }
            println!("edges:");
            for e in v["edges"].as_array().unwrap() {
                let dir = if e["symmetric"].as_bool().unwrap_or(false) {
                    "<->"
                } else {
                    "->"
                };
                println!(
                    "  {} --{}{dir} {} {}",
                    e["from"].as_str().unwrap(),
                    e["rel"].as_str().unwrap(),
                    e["to"].as_str().unwrap(),
                    e["note"]
                        .as_str()
                        .map(|n| format!("({n})"))
                        .unwrap_or_default()
                );
            }
        }
        QueryCmd::Trace { .. } => {
            let paths = v["paths"].as_array().unwrap();
            if paths.is_empty() {
                println!(
                    "no path between {} and {}",
                    v["from"].as_str().unwrap(),
                    v["to"].as_str().unwrap()
                );
                return;
            }
            for p in paths {
                let mut s = v["from"].as_str().unwrap().to_string();
                for h in p["hops"].as_array().unwrap() {
                    s.push_str(&format!(
                        " {}[{}] {}",
                        h["direction"].as_str().unwrap(),
                        h["rel"].as_str().unwrap(),
                        h["to"].as_str().unwrap()
                    ));
                }
                println!("({} hops) {s}", p["length"]);
            }
        }
        QueryCmd::Impact { .. } => {
            println!(
                "impact of changing {} — {} affected within depth {} (candidate set, not an oracle)",
                v["changed"].as_str().unwrap(),
                v["affected"],
                v["depth"]
            );
            for b in v["buckets"].as_array().unwrap() {
                println!("distance {}:", b["distance"]);
                for e in b["entities"].as_array().unwrap() {
                    let via: Vec<&str> = e["path"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter_map(|h| h["via"].as_str())
                        .collect();
                    println!(
                        "  {:<40} via {}",
                        e["ref"].as_str().unwrap(),
                        via.join(" → ")
                    );
                }
            }
        }
        QueryCmd::Architecture => {
            println!(
                "{} — {} entities, {} relations",
                v["project"].as_str().unwrap(),
                v["entities"],
                v["relations"]
            );
            println!("kinds:");
            for k in v["kinds"].as_array().unwrap() {
                println!("  {:<12} {}", k["kind"].as_str().unwrap(), k["count"]);
            }
            println!("edges (kind-level):");
            for e in v["edges"].as_array().unwrap() {
                println!(
                    "  {} --{}-> {}  ×{}",
                    e["from_kind"].as_str().unwrap(),
                    e["rel"].as_str().unwrap(),
                    e["to_kind"].as_str().unwrap(),
                    e["count"]
                );
            }
        }
        QueryCmd::Schema => {
            if let Some(t) = v["title"].as_str() {
                println!("{t} ({})", v["name"].as_str().unwrap());
            } else {
                println!("{}", v["name"].as_str().unwrap());
            }
            let charter = v["charter"].as_array().unwrap();
            if !charter.is_empty() {
                println!("charter:");
                for c in charter {
                    println!("  - {}", c.as_str().unwrap());
                }
            }
            let exclusions = v["exclusions"].as_array().unwrap();
            if !exclusions.is_empty() {
                println!("exclusions:");
                for c in exclusions {
                    println!("  - {}", c.as_str().unwrap());
                }
            }
            println!("kinds:");
            for k in v["kinds"].as_array().unwrap() {
                println!(
                    "  {:<12} {}",
                    k["kind"].as_str().unwrap(),
                    k["label"].as_str().unwrap_or("")
                );
            }
            println!("relations:");
            for r in v["relations"].as_array().unwrap() {
                let mut flags = vec![r["propagation"].as_str().unwrap().to_string()];
                if r["symmetric"].as_bool().unwrap_or(false) {
                    flags.push("symmetric".into());
                }
                if r["acyclic"].as_bool().unwrap_or(false) {
                    flags.push("acyclic".into());
                }
                println!(
                    "  {:<10} [{}]  {}",
                    r["relation"].as_str().unwrap(),
                    flags.join(", "),
                    r["description"].as_str().unwrap_or("")
                );
            }
        }
    }
}
