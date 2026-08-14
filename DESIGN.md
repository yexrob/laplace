# Laplace — Design

Origin: extracted from [bingo#73](https://github.com/Mrzrb/bingo/issues/73) ("Entity mapping — domain-agnostic single source of truth with graph layer"), repositioned as a standalone, harness-agnostic tool.

## Problem

As a project grows past a complexity threshold, an AI agent's picture of it drifts from reality. Entities live scattered across docs, memory, and whatever the model happens to read this session — there is no authoritative answer to "what exists, and how things connect". Re-deriving the map every session is expensive and inconsistent; agents and humans end up disagreeing on what the project actually is. Humans also lack a good way to *look at* that picture.

## Principles

1. **Fully domain-general — no domain is special-cased.** The schema ships no built-in kinds. Every project defines its own kinds. Nothing in the format, graph engine, view, or summary hardcodes any domain concept. Generality is proven by modeling *different* projects (fixtures in ≥2 domains), not by special-casing a category.
2. **Single source of truth.** `laplace.yaml` is the only writable artifact. The graph is a derived cache; the summary and the HTML view are projections. Nothing else is ever authoritative.
3. **The AI maintains, the human views.** The AI is the writer of the truth — in-session edits under the skill discipline, or a background agent. The human consumes projections (HTML view, injected summary) and hands entity references back. This division is what makes the map affordable to keep fresh.
4. **Harness-agnostic by contract.** Laplace ships interfaces (CLI, MCP, summary text), not harness patches. Any agent harness integrates through the same contracts.
5. **Laplace never writes the truth file** (`init` scaffolding excepted). Every command is read-only over `laplace.yaml`; the agent's editor is the sole writer. This is what keeps YAML comments and formatting alive — nothing ever round-trips through a serializer — and it keeps the tool honest: a projector, not a co-author.

Normative format, validation, and query semantics: [docs/SPEC.md](docs/SPEC.md).

## Architecture

```
domain    schema: user-defined kinds / relations / attrs   sole entry point for domain knowledge
truth     laplace.yaml  ★ the only writable file ★         everything below is a deterministic projection
draft     structure walk → model synthesis → review        the draft machine replaces hand-authoring
index     in-memory property graph                         derived cache, lazily rebuilt (hash check)
query     search / trace / impact / architecture           via CLI and MCP; summary + on-demand ≫ full YAML
consume   summary injection + HTML view                    one truth, two audiences (model / human)
handoff   copy entity string-ref in view → paste to agent  zero write path in the view
```

## The truth file

`laplace.yaml`, at the project root. Multi-document YAML, one entity per document, Backstage-style envelope:

```yaml
apiVersion: laplace/v1
kind: character            # user-defined; declared in the schema preamble
metadata:
  name: qing-luan
  tags: [protagonist]
spec:
  description: ...
  relations:
    appears-in: [chapter:default/ch-03]
    rival-of: [character:default/shen-yu]
  lifecycle: active
```

- **Kinds and relation types are declared per project** in a schema preamble (first document), then validated: unknown kinds, dangling string-refs, malformed envelopes are rejected with actionable messages.
- **String-refs** (`kind:namespace/name`) are the universal join key — used in relations, in the HTML view's copy button, and in human↔agent handoff.
- Versioned, diffable, commentable. The file's own git history (when available) sources the "recent changes" digest.

## Freshness model

Three perception paths, one background channel, one accepted limitation:

- **In-turn self-awareness** (weakest, most semantic): the skill discipline requires the model to update the map in the same turn its edits touch an entity.
- **Cross-session calibration**: at session start the model audits the injected summary against change signals (`git log`/`git diff` since the map's last change, directory tree). Stale summary → refresh before working. The map is an audit subject each session, not a memory to trust.
- **Skeleton drift check** (phase 2, capability-bound): for parseable sources, a tree-sitter symbol layer extracts actual symbols and diffs them against the map's declarations — the only perception that does not depend on model discipline. *Explicitly out of v1 scope; v1 freshness = discipline + git calibration.*
- **Background-agent channel**: any harness may dispatch an agent to refresh the map; the harness's own completion notification closes the loop.
- **Accepted limitation**: structural freshness is guaranteed (file/module level); *semantic* correctness is trusted to the AI and surfaced to the human through the view. A confidently-wrong description is caught only by a human reading it.

## Interfaces

| Interface | Form | Notes |
|---|---|---|
| `laplace init` | CLI | scaffold `laplace.yaml` + schema preamble interactively or from a template |
| `laplace validate` | CLI | schema + ref integrity; CI-friendly exit codes |
| `laplace query <tool>` | CLI | `search`, `get`, `neighbors`, `trace`, `impact`, `architecture`; JSON or text output (SPEC §4) |
| `laplace summary` | CLI | entity index + relation digest + recent changes, **token-capped** (tiered truncation: counts → kind index → per-entity lines); designed to be injected into an agent's system context by the harness (Claude Code: CLAUDE.md snippet or SessionStart hook) |
| `laplace serve` | CLI | read-only HTML view (tiny_http, GET-only) |
| `laplace mcp` | MCP server (stdio) | the query tools for any MCP client; no write tools in v1 |
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

- **M1 — core**: format model, schema preamble + validation, in-memory graph engine, `laplace validate`; two fixture projects in different domains — bingo (codebase) and 西游记·前七回 (narrative, Unicode-native refs)
- **M2 — query**: `laplace query` CLI + `laplace mcp` server; correct results on both fixtures
- **M3 — inject**: `laplace summary` with token cap + the `entity-map` skill; end-to-end: agent maintains the map on a fixture, summary + on-demand query replaces full-YAML reading
- **M4 — view**: `laplace serve`; browse/search/filter/multi-select/copy-ref on both fixtures
- **M5 — drift** (phase 2): tree-sitter symbol skeleton + drift diff for parseable projects

## Naming

Laplace's demon: *an intellect that knows every entity and every force, and can therefore derive any consequence* — the mythological prototype of `impact`. crates.io `laplace` is squatted by a placeholder crate; the binary is `laplace`, a future published crate can be `laplace-cli`. Not a blocker.
