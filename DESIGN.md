# Laplace — Design

Origin: extracted from [bingo#73](https://github.com/Mrzrb/bingo/issues/73) ("Entity mapping — domain-agnostic single source of truth with graph layer"), repositioned as a standalone, harness-agnostic tool.

## Problem

As a project grows past a complexity threshold, an AI agent's picture of it drifts from reality. Entities live scattered across docs, memory, and whatever the model happens to read this session — there is no authoritative answer to "what exists, and how things connect". Re-deriving the map every session is expensive and inconsistent; agents and humans end up disagreeing on what the project actually is. Humans also lack a good way to *look at* that picture.

## Principles

1. **Fully domain-general — no domain is special-cased.** The schema ships no built-in kinds. Every project defines its own kinds. Nothing in the format, graph engine, view, or summary hardcodes any domain concept. Generality is proven by modeling *different* projects (fixtures in ≥2 domains), not by special-casing a category.
2. **Single source of truth.** The vault (one markdown file per entity + `schema.yaml`) is the only authoritative artifact. The graph is a derived cache; the summary and the HTML view are projections. Nothing else is ever authoritative.
3. **The AI maintains, the human views.** The AI is the writer of the truth — in-session edits under the skill discipline, or a background agent. The human consumes projections (HTML view, injected summary) and hands entity references back. This division is what makes the map affordable to keep fresh.
4. **Harness-agnostic by contract.** Laplace ships interfaces (CLI, MCP, summary text), not harness patches. Any agent harness integrates through the same contracts.
5. **Laplace mediates writes; it does not own them.** The agent creates and links entities *through* Laplace's transactional operations (validate-before-write: format errors are impossible by construction, dangling refs are rejected synchronously with did-you-mean), so the vault never goes invalid through the write path. Ownership is split: frontmatter is machine-owned, the markdown body is prose and never touched implicitly. Direct file edits stay legal — `validate` reconciles them. (The Obsidian shape, agent-first: vault of plain files as truth, the app as the operation mediator, links/backlinks/graph computed.)

Normative format, validation, and query semantics: [docs/SPEC.md](docs/SPEC.md).

## Architecture

```
domain    schema.yaml: kinds / relations / charter         the constitution; sole entry point for domain knowledge
truth     vault: one .md per entity  ★ single truth ★      path is identity; frontmatter structured, body prose
ops       add/update/link/unlink/remove/rename             transactional: validate against live graph, then write
draft     structure walk → model synthesis → review        the draft machine replaces hand-authoring
index     in-memory property graph                         derived cache, lazily rebuilt (content-hash check)
query     search/get/neighbors/trace/impact/arch/schema    via CLI and MCP; summary + on-demand ≫ full vault
consume   summary injection + HTML view                    one truth, two audiences (model / human)
handoff   copy entity string-ref in view → paste to agent  zero write path in the view
```

## The truth file

A **vault** directory (default `laplace/`): `schema.yaml` as the constitution (kinds, relation types with reading-direction descriptions and propagation, and the **charter** — the questions the map exists to answer), plus one markdown file per entity at `<kind>/[<namespace>/]<name>.md` — flat frontmatter for structured fields (title, tags, lifecycle, relations, source anchors), markdown body as the prose description.

```markdown
---
tags: [主角]
relations:
  师从: [character:菩提祖师]
  持有: [artifact:如意金箍棒]
source: [chapters/ch0[1-7].md]
---
灵明石猴，拜菩提祖师学得地煞七十二变……
```

- **Path is identity**: kind/namespace/name derive from the file path; duplicates are impossible by construction.
- **String-refs** (`kind:namespace/name`, Unicode-native) are the universal join key — in relations, the view's copy button, and human↔agent handoff.
- Versioned, diffable; per-entity git history feeds the "recent changes" digest and per-entity freshness for free.

## Freshness model

Three perception paths, one background channel, one accepted limitation:

- **In-turn self-awareness** (weakest, most semantic): the skill discipline requires the model to update the map in the same turn its edits touch an entity.
- **Cross-session calibration**: at session start the model audits the injected summary against change signals (`git log`/`git diff` since the map's last change, directory tree). Stale summary → refresh before working. The map is an audit subject each session, not a memory to trust.
- **Skeleton drift check** (phase 2, capability-bound): for parseable sources, a tree-sitter symbol layer extracts actual symbols and diffs them against the map's declarations — the only perception that does not depend on model discipline. *Explicitly out of v1 scope; v1 freshness = discipline + git calibration.*
- **Background-agent channel**: any harness may dispatch an agent to refresh the map; the harness's own completion notification closes the loop.
- **Accepted limitation**: structural freshness is guaranteed (file/module level); *semantic* correctness is trusted to the AI and surfaced to the human through the view. A confidently-wrong description is caught only by a human reading it.
- **Why self-written + self-validated is still sound**: `validate` guards the *formal* layer only ("is this a legal map"), which happens to cover the model's dominant failure modes (hallucinated refs, invented vocabulary, duplicates). *Semantic* truth ("is this map right") is guarded by four loose lines of defense instead: (1) same-commit review — the discipline puts the map diff in the same commit as the change, so wrong relations surface as three reviewable YAML lines rather than unreviewable model beliefs; (2) drift's mechanical stale/uncovered signals; (3) use-as-audit — the injected summary makes every session test the map's claims against reality, and collisions trigger fixes; (4) the human eye on the serve view. Skeleton diff (M5) later adds the only fully objective evidence for parseable sources.

## Interfaces

| Interface | Form | Notes |
|---|---|---|
| `laplace init` | CLI | scaffold `laplace.yaml` + schema preamble interactively or from a template |
| `laplace validate` | CLI | schema + ref integrity; CI-friendly exit codes |
| `laplace add/update/link/unlink/remove/rename` | CLI + MCP | the write operations: transactional, validate-before-write, atomic; `rename` rewrites all inbound refs (SPEC §2) |
| `laplace schema <op>` | CLI + MCP | constitutional operations — add-kind/add-relation/set/rename-kind/rename-relation; renames rewrite every usage vault-wide atomically (SPEC §2) |
| `laplace query <tool>` | CLI | `search`, `get`, `neighbors`, `trace`, `impact`, `architecture`, `schema`; JSON or text output (SPEC §5) |
| `laplace drift` | CLI | session-start freshness audit: stale entities (via `spec.source` anchors) + uncovered changed paths + unanchored ratio (SPEC §5) |
| `laplace export` | CLI | full graph JSON to stdout — the jq/pipeline escape hatch |
| `laplace summary` | CLI | entity index + relation digest + recent changes, **token-capped** (tiered truncation: counts → kind index → per-entity lines); designed to be injected into an agent's system context by the harness (Claude Code: CLAUDE.md snippet or SessionStart hook) |
| `laplace serve` | CLI | read-only HTML view (tiny_http, GET-only) |
| `laplace mcp` | MCP server (stdio) | 16 tools: 7 queries + validate + drift + 6 write operations + schema ops; no raw-file writes — semantic operations only |
| skill | `skill/entity-map/` | the maintenance discipline as an installable agent skill: when to generate/refresh, "update the truth whenever a change touches entities", "the summary is not enough — query, don't guess" |

## HTML view

`laplace serve` renders deterministically from the graph — a projection, never a second truth:

- Overview bar: entity counts, kind distribution, relation counts, last updated
- Entity list with search + filters (kind / tag), multi-select; detail panel (description, spec, dependencies both directions)
- Relationship graph (Mermaid) of the **selected entity's 1–2 hop neighborhood** — never the whole map
- **Copy entity reference** per entity and per selection; the user pastes it into their agent session
- Read-only; no write path of any kind

## Out of scope

- Bidirectional page↔session linkage (SSE, agent replies in the page, POST uplink). The page is a projection; the session is the single interactive surface. Handoff is deliberate human copy-paste.
- Auto-extraction of entities from raw content (that is Cognee/Graphiti's road). Laplace is a *curation* tool: the model does judgment, not enumeration.
- Persistence/incrementality machinery (SQLite, watchers). Full lazy rebuild of a single YAML is milliseconds at any AI-maintained size.

## Prior art

| Project | What it is | Why not it |
|---|---|---|
| **LikeC4** (5.4k★, MIT, active) | Architecture-as-code DSL, user-defined kinds, interactive diagrams, MCP server with 20+ query tools | The closest wheel — but shape-mismatched: diagram-first UX (views are hand-authored DSL) vs. our catalog-first; a custom DSL vs. schema-validatable YAML; Node runtime vs. single static binary; software-architecture framing throughout docs/examples/iconography (zero non-software usage found in the wild). Mechanically it *could* model a novel; nothing in its ecosystem helps you do so. |
| Structurizr (Lite / vNext) | C4 DSL + renderer | Lite is EOL; vNext is Java/Spring; C4-bound model |
| Backstage / Datadog Software Catalog | IDP entity catalogs | The descriptor format is exactly what we borrow; the platforms are React+Postgres-heavy, ops-domain |
| Engram | MD+frontmatter agent knowledge base, MCP | Same "index is a rebuildable cache, file is truth" philosophy — but note-shaped: no kinds, no schema validation, no view |
| Cognee / Graphiti / Mem0 | Auto-extracted KG memory, DB-backed | Extraction, not curation — the opposite philosophy |
| Codebase-Memory / code-graph-mcp (papers) | Tree-sitter AST knowledge graphs via MCP | Code-only, AST-derived, no curated truth. Their graph/query layer is our blueprint; their two hardest parts (implicit call resolution, huge persisted graphs) vanish because relations are declared and the artifact is one file |

## Milestones

- **M1 — core**: vault model (path-is-identity, frontmatter), schema constitution + validation, in-memory graph engine, `laplace validate` + `laplace query` (7 tools, CLI); two fixture vaults hand-authored in different domains — bingo (codebase) and 西游记·前七回 (narrative, Unicode-native refs)
- **M2 — operations & channels**: the write and schema operations (CLI + MCP), `laplace mcp` (16 tools), `laplace drift`, `laplace export`; fixture edits exercised through the ops path
- **M3 — inject**: `laplace summary` with token cap + the `entity-map` skill (drift-aware session-start discipline); end-to-end: agent maintains the map on a fixture, summary + on-demand query replaces full-YAML reading
- **M4 — view**: `laplace serve`; browse/search/filter/multi-select/copy-ref on both fixtures
- **M5 — skeleton** (phase 2): tree-sitter symbol extraction for parseable projects — the objective drift evidence beyond file-level anchors

## Naming

Laplace's demon: *an intellect that knows every entity and every force, and can therefore derive any consequence* — the mythological prototype of `impact`. crates.io `laplace` is squatted by a placeholder crate; the binary is `laplace`, a future published crate can be `laplace-cli`. Not a blocker.
