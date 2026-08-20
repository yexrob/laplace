# Laplace

> The entity map for projects that have outgrown a single context window.

Laplace keeps an authoritative, versioned map of **what exists in a project and how it connects**. An AI agent maintains a vault of Markdown entities, while Laplace validates mediated writes and derives graph queries, compact context, JSON exports, and a read-only web view from that single source of truth.

It is domain-agnostic by design: a codebase may define `module`, `contract`, and `subsystem`; a novel may define `character`, `event`, and `location`. Laplace has no built-in entity kinds.

## Why Laplace?

As a project grows, an agent's understanding drifts across source files, documentation, and past sessions. Reconstructing that understanding repeatedly is expensive, incomplete, and difficult for humans to inspect.

Laplace gives the project one explicit model:

- **Plain files are the truth.** Each entity is a Markdown file with YAML frontmatter; `schema.yaml` defines the project's vocabulary.
- **Relationships are queryable.** Search entities, inspect neighborhoods, trace connections, and calculate change impact without loading the whole vault into context.
- **Writes are transactional.** Add, update, link, remove, and rename operations validate the live graph before touching disk.
- **Projections stay disposable.** The graph, summary, JSON export, and HTML view are deterministic outputs—not competing sources of truth.
- **Agents write; humans inspect.** CLI, MCP, and an installable agent skill support maintenance, while the browser view remains read-only.
- **Unicode works end to end.** Kinds, namespaces, names, tags, relations, and references can use any script.

## Status

Laplace is a working **pre-1.0** implementation. The vault model, validation, graph queries, transactional operations, drift audit, summaries, MCP server, agent skill, and read-only web view are implemented. Interfaces and the vault format may still change before 1.0.

See [DESIGN.md](DESIGN.md) for the rationale and [docs/SPEC.md](docs/SPEC.md) for the normative v1 behavior.

## Install from source

A Rust toolchain with Rust 2024 edition support is required.

```bash
git clone https://github.com/yexrob/laplace.git
cd laplace
cargo install --path .
laplace --version
```

For repository-local development, replace `laplace` in the examples below with `cargo run --`.

## Quick start

### 1. Create a vault

Run this from a project root:

```bash
laplace init --name my-project
```

This creates `laplace/schema.yaml`. Edit its charter, kinds, and relation types before adding entities:

```yaml
apiVersion: laplace/v1
name: my-project
root: ..
charter:
  - 改动一个模块，哪些模块和契约必须跟着重看？

kinds:
  module:
    description: 一个代码模块。描述应说明它的职责和边界。
  contract:
    description: 被其他模块或进程独立消费的契约。

relations:
  depends-on:
    description: A depends-on B —— A 是消费方，B 是被依赖方。
    propagation: to-source
    from: [module]
    to: [module, contract]
```

The schema is the vault's constitution. It defines the vocabulary; Laplace does not assume what a project contains.

### 2. Add and connect entities

```bash
laplace add module api \
  --body "对外 API 入口，负责请求解析与响应组装。"

laplace add contract http-api \
  --body "供外部客户端调用的 HTTP 接口契约。"

laplace link module:api depends-on contract:http-api
laplace validate
```

Each entity becomes a plain Markdown file:

```markdown
---
relations:
  depends-on: [contract:http-api]
---
对外 API 入口，负责请求解析与响应组装。
```

Identity comes from the path: `laplace/module/api.md` is `module:api`. Backlinks are computed and never stored twice.

### 3. Ask graph-shaped questions

```bash
# Resolve a name before guessing or creating a reference
laplace query search api

# Read an entity, including incoming and outgoing edges
laplace query get module:api

# Inspect the local graph
laplace query neighbors module:api --depth 2

# Find shortest annotated paths between two entities
laplace query trace module:api contract:http-api

# Produce a distance-bucketed review list for a change
laplace query impact contract:http-api --depth 2

# Get a kind-level overview of the whole project
laplace query architecture
```

Add `--json` before the subcommand for machine-readable output:

```bash
laplace --json query impact contract:http-api | jq
```

### 4. Open the read-only view

```bash
laplace serve --port 6174
```

Then open <http://127.0.0.1:6174>. The view is derived from the vault and has no write path.

## Vault format

```text
laplace/
├── schema.yaml
├── module/
│   ├── api.md
│   └── storage.md
├── contract/
│   └── http-api.md
└── subsystem/
    └── request-path.md
```

An entity lives at `<kind>/<name>.md`, or `<kind>/<namespace>/<name>.md` when a namespace is needed. References use the same identity:

```text
module:api
character:龙宫/敖广
```

Entity frontmatter may contain:

- `title`: optional display name;
- `tags`: optional labels;
- `lifecycle`: optional project-defined state;
- `relations`: declared relation types and target references;
- `source`: root-relative globs used by drift detection;
- additional project-defined keys.

The Markdown body is prose. Field operations preserve it unless `--body` is explicitly supplied. See [docs/SPEC.md](docs/SPEC.md) for the complete format, reference grammar, propagation semantics, and validation rules.

## CLI overview

| Command | Purpose |
|---|---|
| `init` | Scaffold a new `laplace/schema.yaml` |
| `validate` | Check structure, declarations, references, relation constraints, and source anchors |
| `query search|get|neighbors|trace|impact|architecture|schema` | Query the derived graph |
| `add`, `update`, `link`, `unlink`, `remove`, `rename` | Perform validated entity transactions |
| `schema add-kind|add-relation|set|rename-kind|rename-relation` | Change the vault constitution transactionally |
| `drift` | Compare source anchors with Git changes and report stale or uncovered territory |
| `summary` | Render a token-budgeted context block for an agent harness |
| `export` | Write the complete graph as JSON to stdout |
| `serve` | Start the read-only HTML view |
| `mcp` | Start the MCP server over stdio |
| `skill show|install` | Print or install the bundled `entity-map` agent skill |

Use `laplace <command> --help` for all flags and input forms. Entity and schema mutation commands return exit code `1` when an operation is rejected; top-level usage and setup failures return `2`.

## Agent integration

### MCP

Configure an MCP client to start Laplace over stdio with an explicit vault:

```json
{
  "mcpServers": {
    "laplace": {
      "command": "laplace",
      "args": [
        "--vault",
        "/absolute/path/to/project/laplace",
        "mcp"
      ]
    }
  }
}
```

The server exposes graph queries, validation and drift checks, transactional write operations, schema edits, and the session-scoped web view. To serve several vaults from one process, scan a directory instead:

```bash
laplace mcp --scan /path/to/projects
```

Use `laplace_vaults` to list discovered vaults; tool calls accept a `vault` selector when more than one is available.

### Agent skill

Laplace ships an `entity-map` skill that teaches an agent when to query, validate, and update the map:

```bash
laplace skill show
laplace skill install
```

Installation detects existing Claude Code and bingo skill directories, or accepts an explicit destination:

```bash
laplace skill install --to ~/.config/my-harness/skills
```

### Context summary

Generate a compact, token-budgeted map for session-start injection:

```bash
laplace summary --budget 1200
```

The summary is a table of contents, not a replacement for graph queries. `laplace drift` should be run before relying on it in a new session.

## Example vaults

The repository includes two substantive fixtures that exercise the same engine in different domains:

- [`fixtures/bingo/laplace`](fixtures/bingo/laplace) models a Rust agent codebase with modules, subsystems, contracts, and invariants.
- [`fixtures/xiyouji/laplace`](fixtures/xiyouji/laplace) models the first seven chapters of *Journey to the West* with characters, artifacts, locations, events, and chapters.

Try them without modifying the repository's own vault:

```bash
laplace --vault fixtures/xiyouji/laplace validate
laplace --vault fixtures/xiyouji/laplace query search 孙悟空
laplace --vault fixtures/xiyouji/laplace query impact artifact:如意金箍棒 --depth 1
laplace --vault fixtures/xiyouji/laplace serve
```

## Design boundaries

Laplace is deliberately a **curated entity map**, not an automatic knowledge-extraction system.

It does not:

- infer semantic truth from source files;
- hardcode software architecture or any other domain ontology;
- store a second database beside the Markdown vault;
- provide writes through the browser view;
- promise that impact analysis is complete beyond the relationships the vault declares.

The agent supplies judgment; Laplace supplies structure, validation, graph semantics, and reproducible projections.

## Development

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

Repository guide:

```text
src/                Rust library and CLI implementation
tests/              Integration tests and intentionally broken vaults
docs/SPEC.md        Normative v1 specification
DESIGN.md           Design rationale, scope, and milestones
skill/entity-map/   Bundled agent-maintenance skill
fixtures/           Cross-domain example vaults
laplace/            This repository's own entity map
```

## License

[MIT](Cargo.toml)
