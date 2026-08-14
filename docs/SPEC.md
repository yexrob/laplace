# Laplace SPEC (v1)

Normative specification of the truth file format, validation, graph semantics, query tools, and projections. DESIGN.md holds the why; this holds the what. Language of examples: any — Laplace is Unicode-native.

## 0. Invariants

1. **Single truth.** `laplace.yaml` is the only authoritative artifact. Graph, summary, view are derived, deterministic, and disposable.
2. **Laplace never writes the truth file.** Every command is read-only over `laplace.yaml` (`laplace init` scaffolds it only when absent). The agent's editor is the sole writer. Consequence: YAML comments, key order, and formatting are always preserved, because nothing ever round-trips the file through a serializer.
3. **No built-in kinds.** All kinds and relation types are declared per project in the Schema preamble. The only reserved word is the kind `Schema` itself.
4. **Unicode-native identifiers.** Names, kinds, namespaces, and tags may be any script. Refs are for copy-paste, not for typing.

## 1. The truth file

### 1.1 Layout

A single multi-document YAML file, `laplace.yaml`, at the project root. Document 0 MUST be the Schema preamble (`kind: Schema`); every following document is one entity. Discovery: commands look for `laplace.yaml` in the working directory, then ancestors (git-style); `--file PATH` overrides.

### 1.2 Envelope

Every document:

```yaml
apiVersion: laplace/v1   # required on the Schema doc; optional on entities (inherited; if present, must match)
kind: <declared-kind>
metadata:
  name: <word>           # required; unique within (kind, namespace)
  namespace: <word>      # optional, default "default"
  title: <string>        # optional display name
  tags: [<word>...]      # optional
spec:
  description: <string>  # optional but the skill demands it
  lifecycle: <string>    # optional free string; skill recommends a small per-project vocabulary
  relations: {...}       # optional; see 1.4
  source: [<glob>...]    # optional: project-relative globs anchoring this entity to files (feeds drift, §5)
  <anything else>: ...   # free-form, preserved, displayed; not validated in v1
```

Unknown keys inside `metadata`/`spec` are preserved and shown (spec is open). Unknown top-level keys are an error (`bad-envelope`).

### 1.3 Refs

```
ref  = kind ":" [ namespace "/" ] name
word = 1+ of { Unicode letters, Unicode digits, "-", "_", "." }
```

`kind`, `namespace`, `name` are words. Forbidden in words: whitespace, `:`  `/`  `,`  `[`  `]`, quotes. Namespace elision: `character:孙悟空` ≡ `character:default/孙悟空`. Tools always emit the canonical (namespaced) form. Matching is exact (no case folding — casing is part of identity; search is where fuzziness lives).

### 1.4 Relations

`spec.relations` maps a declared relation type to a list of targets. Each entry is either a bare ref or an object carrying edge attributes:

```yaml
spec:
  relations:
    持有:
      - artifact:default/如意金箍棒
      - ref: artifact:default/凤翅紫金冠
        note: 龙宫索得，非战斗所用
```

Object form: `ref` required; `note` and any other keys are free-form edge attributes, preserved and displayed. Duplicate edges (same source, type, target) are deduplicated with a warning.

### 1.5 Schema preamble

```yaml
apiVersion: laplace/v1
kind: Schema
metadata:
  name: xiyouji
  title: 西游记·前七回
spec:
  kinds:
    character: { description: 人物 }
    artifact:  { description: 法宝与兵器 }
    chapter:   { description: 回目 }
  relations:
    师从:     { propagation: to-target }
    持有:     { propagation: to-source }
    出现于:   { propagation: to-target }
    宿敌:     { symmetric: true, propagation: none }
```

- `spec.kinds`: map of kind → `{description?}`. An entity whose kind is absent here → `unknown-kind`.
- `spec.relations`: map of relation type → declaration. A relation used but not declared → `undeclared-relation`.

### 1.6 Propagation (the impact semantics contract)

For an edge `A --rel--> B` (A's spec declares rel targeting B), the declaration's `propagation` states where a change propagates:

| value | meaning | intuition |
|---|---|---|
| `to-source` *(default)* | a change to **B** affects **A** | dependency: A 持有 B; B (法宝) changes → A (人物) affected |
| `to-target` | a change to **A** affects **B** | containment/appearance: A 出现于 B; A (人物) changes → B (回目) affected |
| `both` | either end affects the other | tight coupling |
| `none` | no impact propagation | pure annotation (宿敌) |

`symmetric: true` additionally means direction is meaningless: the graph materializes the edge in both directions, and `propagation` must be `both` or `none` (else `bad-propagation`).

This is why impact works across domains: the *domain layer* (Schema) declares causality direction per relation type; the engine stays generic.

## 2. Validation

Three layers, all hard errors unless noted:

1. **Structure**: YAML parses; envelope well-formed; ref syntax valid; doc 0 is Schema; `Schema` kind unused by entities (`reserved-kind`); no duplicate `(kind, namespace, name)` (`duplicate-entity`).
2. **Declaration**: every entity kind ∈ `spec.kinds`; every relation type ∈ `spec.relations`; propagation values legal.
3. **Reference**: every relation target resolves to an existing entity (`dangling-ref`). Dangling refs get a did-you-mean when a candidate is close: same name under another kind/namespace, or edit distance ≤ 2 on the name.

Error shape (also `--json`): `{severity, code, doc, entity?, path?, message, suggestion?}`, addressed by document index and canonical ref — e.g.

```
laplace.yaml doc 17 (character:default/沈雨): relations.宿敌 → character:default/沈玉
  dangling-ref: no such entity. Did you mean character:default/沈雨?
```

Warnings: `duplicate-edge` (deduped), `empty-map`. Exit codes: 0 clean (warnings allowed), 1 errors, 2 usage/IO.

## 3. Graph model

- **Node** = entity, keyed by canonical ref. **Edge** = `(source, type, target, attrs)`. Symmetric types materialize both directions. Reverse adjacency is always built.
- **Propagation digraph** (derived): for each edge per its declaration — `to-source` contributes step target→source; `to-target` contributes source→target; `both` contributes both; `none` contributes none. `impact` runs on this digraph; everything else runs on the plain graph.
- **Lifecycle**: the graph is a lazy in-memory cache. Long-running processes (`serve`, `mcp`) re-check `(mtime, xxhash)` of the file per request and rebuild on change. No persistence, no watcher, no incremental sync. An invalid file: `validate` reports; query/serve/mcp fail fast with the same diagnostics (no queries over a broken truth).

## 4. Query tools

Seven tools; one semantics shared by CLI (`laplace query <tool>`, human text default, `--json` for machines) and MCP (`laplace_<tool>`, always JSON).

| tool | signature | returns |
|---|---|---|
| `search` | `q, kind?, tag?, limit=20` | ranked refs with matched-field markers. Score = max of: name-exact 100, name-prefix 80, name-substring 60, title-substring 50, tag-exact 40, description-substring 20. Unicode-casefolded. Ties: kind, then ref. |
| `get` | `ref` | the full document + computed edges, both directions, with attrs |
| `neighbors` | `ref, depth=1 (max 2), kinds?, relations?` | induced subgraph (nodes + edges) around ref — the serve view's data source |
| `trace` | `from, to, limit=5, max_len=6` | shortest simple paths in the undirected view, each hop annotated `(relation, direction)` — answers "how are these two connected?" |
| `impact` | `ref, depth=10, via?` | BFS closure over the propagation digraph: `{ref, distance, path}` per affected entity, one shortest witness path each, sorted by distance — answers "what does changing this touch?" `via` restricts relation types. |
| `architecture` | — | kind-level condensation: `{kind, count}` nodes, `{from_kind, type, to_kind, count}` aggregated edges — the whole-map overview that IS safe to render |
| `schema` | — | the parsed Schema preamble: declared kinds and relation types with propagation/symmetric — the maintainer's cheat sheet before writing new entries |

MCP tool descriptions are part of the contract: each teaches its when-to-use in one line (e.g. `laplace_search`: "resolve names to refs — search before adding an entity or guessing a ref"). The MCP server also exposes `laplace_validate` and `laplace_drift` (below) so an agent can close its maintenance loop without shell access. All MCP tools are read-only — the write invariant (§0.2) has no MCP exception.

## 5. Drift (cross-session calibration as a tool)

`laplace drift [--since REV] [--json]` — turns the session-start freshness audit into one call. Requires a git repository; entities participate by declaring `spec.source` globs.

- **Base**: the last commit touching `laplace.yaml` (`--since` overrides).
- **Changed set**: paths changed in commits after base, plus dirty working-tree paths (`git status --porcelain`).
- **Report**:
  - `stale` — entities whose `source` globs match changed paths: `{ref, paths, commits}` (the map may misdescribe these);
  - `uncovered` — changed paths matching no entity's globs (candidate new/unmapped territory);
  - `unanchored` — count and ratio of entities with no `source` at all (coverage disclosure, so silence is never mistaken for cleanliness).
- Exit 0 always (informational). No git or zero anchors → an explicit notice, never a silent pass.

Domain note: anchoring is capability-bound, not domain-bound — a novel anchors `character:孙悟空` to `chapters/ch0[1-7].md` exactly as a codebase anchors `module:core/term` to `src/term/**`. Projects with no file-shaped source simply stay on discipline + review.

## 6. Summary projection

`laplace summary [--budget N]` (default budget 1200 tokens) emits the block a harness injects into agent context:

```
<laplace-map project="西游记·前七回" entities="54" relations="121" updated="2026-08-14">
kinds: character(21), artifact(9), location(8), event(12), chapter(4)
relation-types: 师从(12), 持有(9), 出现于(74), 宿敌(3), ...
character: 孙悟空, 菩提祖师, 玉皇大帝, 太白金星, …+7 more
artifact: 如意金箍棒, 凤翅紫金冠, …
recent:
  2026-08-14 map: 收录大闹天宫新增事件
  2026-08-13 map: 初版入册
This map is authoritative. Details: `laplace query get <ref>`; do not guess beyond it.
If your edits touch an entity (add/rename/remove/re-relate), update laplace.yaml in the same turn.
</laplace-map>
```

- **Tiers**: T0 header+kinds+discipline footer → T1 +relation-types → T2 +recent (last 5 commits touching the file; omitted without git) → T3 +per-kind name lists. Fitting: start at T3; if over budget, truncate name lists (largest kind first) with `…+N more`; then drop tiers T3→T2→T1. T0 always emits.
- **Token estimation** is CJK-aware: `ceil(ascii/4) + cjk + ceil(other/2)` — CJK counts ~1 token per char; ASCII-calibrated ÷4 heuristics undercount it badly (learned the hard way in bingo#40).
- The two discipline lines are part of the format contract — harnesses and the skill rely on their exact presence.

## 7. CLI surface

```
laplace init                 scaffold laplace.yaml (only if absent) with a commented Schema template
laplace validate [--json]
laplace query <tool> …       see §4
laplace drift [--since REV]  see §5
laplace summary [--budget N]
laplace export               full graph JSON to stdout (same payload as /api/graph) — the jq/pipeline escape hatch
laplace serve [--port 6174]
laplace mcp                  MCP server on stdio
```

Deliberate non-tools: no write/`fmt`/`mv` commands (write invariant §0.2 — renames are agent edits, `validate`'s did-you-mean is the net); no `watch` (lazy rebuild makes it pointless); no standalone `stats` (`architecture` is it).

Global: `--file PATH`. `serve` v1: GET-only (tiny_http); routes `/` (embedded single-page shell) and `/api/graph` (full graph JSON; client renders list/detail/filters, Mermaid draws `neighbors` of the selection only). Default port 6174 — the Kaprekar constant: every start converges to the same fixed point, like the map.

## 8. Versioning & compatibility

- `apiVersion: laplace/v1`. Additive evolution within v1 (new optional envelope fields, new declaration fields). Breaking format changes bump to v2; readers refuse unknown majors with a clear message.
- Reserved now, undefined until needed: kind `Schema` (only), `metadata.annotations`, per-kind attribute schemas (`spec.kinds.<k>.attributes`), directory-of-files loading (the format is already document-level; only discovery changes).

## 9. Implementation notes (non-normative)

- YAML: `serde_yaml` is archived — evaluate the active successors (`saphyr`-based serde bridges, `serde_norway`, `serde_yml`) at implementation time; requirement: multi-doc streams + serde derive + decent error positions.
- MCP: hand-rolled stdio JSON-RPC (~small) vs the official `rmcp` SDK — decide at M2; the tool surface (§4) is fixed either way.
- Graph: hand-rolled adjacency maps; no petgraph until an algorithm actually demands it.
- Did-you-mean: Levenshtein over names within the same kind first, then across kinds; suggest at distance ≤ 2 only.
- Rust modules: `model` (envelope+refs+schema), `load` (parse+validate+diagnostics), `graph`, `query`, `summary`, `serve`, `mcp`, thin `main` (clap).
