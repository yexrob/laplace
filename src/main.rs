//! laplace — the intellect that knows every entity in your project and how they connect.

use anyhow::Result;
use clap::{Parser, Subcommand};
use laplace::graph::Graph;
use laplace::{query, validate, vault};
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
    /// Validate the vault: structure, declarations, references, anchors.
    Validate,
    /// Query the graph.
    #[command(subcommand)]
    Query(QueryCmd),
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
    let dir = vault::discover(&cwd, cli.vault.as_deref())?;
    let vault = vault::load(&dir)?;
    let report = validate::run(&vault);

    match cli.cmd {
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
    }
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
