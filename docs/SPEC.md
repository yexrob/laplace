# Laplace SPEC (v1)

Normative specification of the vault format, write operations, validation, graph semantics, query tools, and projections. DESIGN.md holds the why; this holds the what. Laplace is Unicode-native throughout.

The shape of the system: a **vault** of plain entity files (the truth), and **Laplace as the app that mediates operations on it** — creation and linking are validated transactions, backlinks and the graph are computed, projections are derived. The agent is the app's primary user; the human views.

## 0. Invariants

1. **Single truth.** The vault is the only authoritative artifact. Graph, summary, view are derived, deterministic, disposable.
2. **Ownership split.** Frontmatter (structured fields) is machine-owned — written through Laplace's operations. The markdown body is prose — never touched except by an explicit description edit. Direct file edits remain legal (human emergencies, merge resolution, bulk refactors); `validate` reconciles them.
3. **Transactional writes.** Every write operation validates against the live graph *before* touching disk; a failed operation writes nothing and returns actionable errors. Through Laplace's write path, the vault never enters an invalid state.
4. **No built-in kinds.** All kinds and relation types are declared in `schema.yaml`. Nothing anywhere hardcodes a domain.
5. **Unicode-native identifiers.** Kinds, namespaces, names, tags in any script. Refs are for copy-paste, not typing.

## 1. The vault

### 1.1 Layout

```
laplace/
├─ schema.yaml            # the constitution: kinds, relation types, charter
├─ character/
│  ├─ 孙悟空.md
│  └─ 菩提祖师.md
├─ character/龙宫/         # non-default namespace = one more directory level
│  └─ 敖广.md
└─ artifact/
   └─ 如意金箍棒.md
```

- **Path is identity**: `<kind>/<name>.md`, or `<kind>/<namespace>/<name>.md` for a non-default namespace. Kind, namespace, and name are derived from the path and never repeated in frontmatter. Duplicate entities are impossible by construction (the filesystem enforces uniqueness); a wrongly moved file changes identity and surfaces as dangling refs in `validate`.
- **Discovery**: from cwd upward, the first directory containing `laplace/schema.yaml`; `--vault DIR` overrides (requires `DIR/schema.yaml`, so a vault may also be a project root itself).

### 1.2 Entity file

Markdown with YAML frontmatter. Frontmatter is flat — no metadata/spec nesting:

```markdown
---
title: 孙悟空          # optional display name (defaults to the file name)
tags: [主角, 妖仙]     # optional
lifecycle: active      # optional free string; schema may recommend a vocabulary
relations:
  师从: [character:菩提祖师]
  持有:
    - ref: artifact:如意金箍棒
    - ref: artifact:凤翅紫金冠
      note: 龙宫索得，非战斗所用
source: ["chapters/ch0[1-7].md"]   # optional drift anchors: globs MUST be quoted (brackets are YAML flow syntax)
---
灵明石猴，东胜神洲花果山出身。拜菩提祖师学得地煞七十二变与筋斗云，后大闹天宫，自号齐天大圣。

正文即描述：自由散文，属于人可珍视的层，工具的字段级操作永不触碰。
```

- The body is the description. Its **first sentence** (up to the first sentence terminator, capped at 120 display cells) is the one-line summary in query results and views — "first line" does not survive CJK prose, which is naturally unwrapped; author the first sentence to be summary-worthy.
- Unknown frontmatter keys are free-form, preserved, and displayed — the format is open at the attribute level.
- **The map asserts the present.** 敖广曾持有金箍棒 is prose (or an edge `note`), not a present-tense `持有` edge; the past lives in the body and in the vault's own git history — that history *is* the time axis, so there is no temporal edge model (§9).

### 1.3 Refs

```
ref  = kind ":" [ namespace "/" ] name
word = 1+ of { Unicode letters, Unicode digits, "-", "_", "." }
```

`kind`, `namespace`, `name` are words; forbidden characters: whitespace, `:` `/` `,` `[` `]`, quotes (names are also filesystem-constrained by construction). Namespace elision: `character:孙悟空` ≡ `character:default/孙悟空`. Tools emit the canonical namespaced form. Matching is exact — search is where fuzziness lives — and ref↔path matching is **NFC-normalized** (macOS filesystems may hand back decomposed forms). The storage suffix is always exactly one appended `.md`: a name may contain dots (`v1.2` → `v1.2.md`).

A namespace is a **one-level organizational disambiguator with project-defined meaning** (a faction, a source directory, a team) — it is not a path mirror; deeper trees flatten into the name.

### 1.4 Relations

`relations:` maps a declared relation type to a list of targets; each entry is a bare ref or an object (`ref` required; `note` and other keys are free edge attributes). Duplicate edges (same source, type, target) are deduplicated with a warning. Relations live only on the source entity's frontmatter; backlinks are computed, never stored.

**Symmetric relations are declared once, on either side.** `A 结义 B` on A's file and `B 结义 A` on B's file are the same edge stated twice — materialized once, flagged `symmetric-declared-twice` (the duplicate-edge rule alone would miss it, since the sources differ).

### 1.5 schema.yaml (the constitution)

```yaml
apiVersion: laplace/v1
name: xiyouji
title: 西游记·前七回
root: ..                              # what source/ignore globs are relative to (default ..: the directory containing the vault)
charter:                              # the questions this map exists to answer
  - 杀掉或改写一个角色，哪些章回和伏笔要重看？
  - 一件法宝的设定变了，波及哪些人物与事件？
ignore: ["notes/**", "**/*.lock"]     # declared non-territory: no entity will ever claim these paths
exclusions:                           # concept-shaped non-goals (ignore is path-shaped; this is its conceptual counterpart)
  - 逐场打斗不入图——事件粒度到主线节点为止
kinds:
  character: { description: 人物。描述应涵盖出身、能力、动机与主要羁绊。 }
  artifact:  { description: 法宝与兵器 }
  chapter:   { description: 回目 }
relations:
  师从:
    description: A 师从 B —— A 是徒弟，B 是师父。改写师父的道法，徒弟必须重看。
    propagation: to-source
    from: [character]
    to: [character]
  持有:
    description: A 持有 B —— A 是持有者，B 是器物。
    propagation: to-source
    from: [character]
    to: [artifact]
  出现于:
    description: A 出现于 B —— A 是人物或器物，B 是章回。
    propagation: to-target
    to: [chapter]
  宿敌:
    description: A 与 B 互为宿敌。对称，不传播。
    symmetric: true
    propagation: none
```

- `root`: the base every `source` and `ignore` glob resolves against, stated relative to the vault directory. Default `..` (the directory containing the vault); a vault that *is* the project root declares `root: .`. One glob string, one meaning — discovery mode never changes it.
- `charter`: the map's reason for existing, as questions. Vocabulary changes must cite a charter question (or add one); `query schema` returns it so every future agent sees why the map is shaped as it is. Charter questions may be impact-shaped ("动它波及什么") *or* connection-shaped ("这俩怎么连的") — the latter are served by `trace`/`neighbors`, and justify `propagation: none` relations.
- `ignore`: declared non-territory — root-relative globs no entity will ever claim; `drift` excludes them from its uncovered report. What the map deliberately does not cover is constitutional knowledge too.
- `exclusions`: concept-shaped non-goals with their reasons — the conceptual counterpart of path-shaped `ignore` ("decision records stay in notes/research.md, they would dominate the graph"). Returned by `query schema`, so deliberate absences stop living only in session transcripts.
- `kinds`: kind → `{description?}`. An entity directory not declared here → `unknown-kind`. Convention: the description's **first sentence** is the display label; the rest is the authoring guide (what a good description of this kind covers), surfaced by the skill at `add` time.
- `relations`: type → declaration. **`description` is required** and must state the reading direction ("A rel B means…") — direction confusion corrupts silently and even the designer gets it wrong unaided; `propagation` defaults to `to-source`; `symmetric: true` requires propagation `both` or `none` (else `bad-propagation`); optional `from:` / `to:` (kind lists) constrain endpoints — an edge violating them is `bad-endpoint`, absent means unconstrained; optional `acyclic: true` declares that the relation may not form cycles (承接-style succession) — a cycle is then an error. A relation used but not declared → `undeclared-relation`.

### 1.6 Propagation (the impact semantics contract)

For an edge `A --rel--> B`, `propagation` states where a change propagates:

| value | meaning | intuition |
|---|---|---|
| `to-source` *(default)* | a change to **B** affects **A** | dependency: 孙悟空 持有 金箍棒 — the weapon changes, the wielder must be revisited |
| `to-target` | a change to **A** affects **B** | containment/appearance: 孙悟空 出现于 第五回 — the character changes, the chapter must be revisited |
| `both` | either affects the other | tight coupling |
| `none` | no propagation | pure annotation (宿敌) |

**The two-question test** (normative guidance for declaring): for `A rel B`, ask "B changed — must A be revisited?" (yes → includes `to-source`) and "A changed — must B be revisited?" (yes → includes `to-target`). Both yes → `both`; both no → `none`.

Three consequences worth knowing before they surprise you:

- **The test is per type, and that is deliberate.** If a type's edges honestly split on the test (五行山↔五行山镇压 is `both`; 花果山↔石猴出世 is `to-source`), that heterogeneity *is* the evidence that one name is wearing two verbs — split the type. Per-edge propagation overrides are rejected (§9): they would dissolve the economy of concentrating semantic judgment at the type level.
- **Pure-sink kinds are normal, not broken.** A container kind reached only by `to-target` edges (chapters) has an empty `impact` closure — "改写第三回，什么要重看" is a *connection-shaped* question answered by `neighbors 第三回`, not `impact`. The skill teaches which tool fits which question shape.
- **`both` is expensive and honest.** In a densely-`both` vocabulary the closure reaches most of the map within 2–3 hops (measured on both fixture domains); that is the true answer, and it is only usable because distance is reported. Do not downgrade an honest `both` to make queries prettier — sacrifice was never the deal; rely on depth and distance instead.

## 2. Write operations

The agent's creation path. Six operations, identical semantics over CLI and MCP; every one validates against the live graph before writing, writes atomically, and on failure writes nothing:

| op | does | validation before write |
|---|---|---|
| `add` | create an entity: kind, name, namespace?, title?, tags?, body (description), relations?, source? | kind declared; ref grammar; no existing entity at that path; every relation type declared; every target resolves (dangling → reject + did-you-mean) and satisfies endpoint constraints |
| `update` | set/unset fields of an existing entity (title, tags, lifecycle, free keys; `--body` replaces the description explicitly) | field-level; body untouched unless explicitly given |
| `link` | add one relation entry `from rel to [--note]` | type declared; both ends resolve; endpoints legal; duplicate edge → no-op warning. Success output echoes the source's full edge list of that type (`now 师从: [菩提祖师, 唐僧]`) — plausibility judgment stays with the model, but the facts it needs are pushed in front of it at the moment of action. Cardinality is deliberately not declared or enforced: category errors (endpoints) get hard rejection, plausibility deviations are often reality itself (孙悟空 has two masters) — the map records what is |
| `unlink` | remove one relation entry | edge exists |
| `remove` | delete an entity | **refuses if inbound refs exist**, listing them (unlink first) — danglings are impossible by construction |
| `rename` | rename/move an entity (name and/or namespace), atomically rewriting **all inbound refs** across the vault | target path free; reports the count of prose *mentions* of the old name in bodies (bodies are never rewritten — prose is human domain; the agent reviews those by hand) |

**Schema operations** — constitutional changes are vault-wide transactions and get the same mediation: `laplace schema add-kind|add-relation|set|rename-kind|rename-relation`. `add-relation` enforces the reading-direction description at creation time; `set` re-checks propagation/symmetric legality; the renames atomically rewrite every usage across entity frontmatters (`rename-kind` also moves the kind directory). `schema.yaml` is machine-owned like frontmatter: ops re-serialize it canonically (comments and scalar styles are not preserved — constitutional prose lives in `description`/`charter`/`exclusions` fields, which survive). Direct schema edits remain legal; `validate` reconciles.

Input forms: CLI flags for the simple cases, `--stdin` JSON for full payloads; MCP tools take structured params (the schema family is one MCP tool, `laplace_schema_edit`, with an `op` discriminator — `laplace_schema` is the read-side constitution query). Multi-file transactions (renames) write all temp files first, then rename all; git is the rollback of last resort.

**Addressability**: refs are paths (`character:孙悟空` ⇔ `laplace/character/孙悟空.md` — locating an entity is a pure function, never a search); `get` returns the entity's vault path; the constitution stays small by vocabulary budget, so YAML key paths (`relations.师从.propagation`) address it; diagnostics carry `file:line`.

What this buys: the model does judgment (what to name, what to connect, why), the machine does syntax (envelope shape is impossible to get wrong — the tool serializes), and ref errors are caught synchronously at write time instead of by a later lint.

## 3. Validation

`laplace validate` is the reconciler for whatever bypassed the write path (direct edits, merges) and the CI gate. Three layers, hard errors unless noted:

1. **Structure**: frontmatter parses; known layout (`kind/[ns/]name.md`); ref grammar.
2. **Declaration**: entity's kind directory ∈ `schema.kinds`; every relation type ∈ `schema.relations`; propagation values legal; relation declarations carry descriptions; edges satisfy endpoint constraints (`bad-endpoint`).
3. **Reference**: every target resolves; danglings get did-you-mean (same name under another kind/namespace, or edit distance ≤ 2).

Errors are file-and-line-addressed: `{severity, code, file, line?, path?, message, suggestion?}` — e.g.

```
laplace/character/沈雨.md:6: relations.宿敌 → character:沈玉
  dangling-ref: no such entity. Did you mean character:沈雨?
```

Two checks added by fixture experience: **`dead-anchor`** — a `source` glob that matches nothing (a renamed file otherwise degrades silently to "never stale", i.e. permanently clean); and **cycle detection** on relations declared `acyclic: true`.

Warnings: `duplicate-edge` (deduped), `symmetric-declared-twice` (deduped), `orphan` (an entity with no edges in either direction), `empty-vault`. Exit codes: 0 clean, 1 errors, 2 usage/IO.

## 4. Graph model

- **Node** = entity (keyed by canonical ref; carries title, tags, lifecycle, one-line, free attrs). **Edge** = `(source, type, target, attrs)`; symmetric types materialize both directions; reverse adjacency always built.
- **Propagation digraph** (derived per §1.6): `impact` runs on it; everything else on the plain graph.
- **Lifecycle**: lazy in-memory cache keyed by a vault content hash (sorted `(path, xxhash)` walk). Long-running processes (`serve`, `mcp`) re-check per request; rebuild is a full re-parse — milliseconds at any AI-maintained size. No watcher, no persistence, no incremental sync. Invalid vault → queries fail fast with the validate diagnostics (no queries over a broken truth).

## 5. Query tools

Seven tools; one semantics shared by CLI (`laplace query <tool>`, human text default, `--json`) and MCP (`laplace_<tool>`, always JSON).

| tool | signature | returns |
|---|---|---|
| `search` | `q, kind?, tag?, limit=20` | ranked refs; score = max of name-exact 100, name-prefix 80, name-substring 60, title-substring 50, tag-exact 40, body-substring 20; Unicode-casefolded; ties by kind then ref |
| `get` | `ref` | frontmatter + full body + computed edges both directions with attrs + the entity's vault path |
| `neighbors` | `ref, depth=1 (max 2), kinds?, relations?` | induced subgraph around ref — the serve view's data source |
| `trace` | `from, to, limit=5, max_len=6` | shortest simple paths, undirected view, hops annotated `(type, direction)` — "how are these two connected?" |
| `impact` | `ref, depth=2, via?` | BFS closure over the propagation digraph, **distance-bucketed** (`distance 1: …` / `distance 2: …`) with one shortest witness path each — "what does changing this touch?" Output is a **candidate set, not an oracle**: sound w.r.t. the declared map, complete never. Default depth 2 because measured closures reach most of a real map by depth 3–4 — **distance is the whole signal**; an impact listing without it is unusable |
| `architecture` | — | kind-level condensation: `{kind, count}` nodes, `{from_kind, type, to_kind, count}` edges — the whole-map overview that is safe to render, and the usage precedent for vocabulary choices |
| `schema` | — | the constitution: charter, kinds, relation types with descriptions and propagation — the agent's first stop before writing |

MCP tool descriptions are contract: each carries a one-line when-to-use (e.g. `laplace_search`: "resolve names to refs — search before adding or guessing"). MCP exposes the seven queries, `laplace_validate`, `laplace_drift`, the six write operations, `laplace_schema_edit` (§2), `laplace_vaults`, and `laplace_serve` (start or reuse the session-scoped view thread for a vault and return its URL — it dies with the session, no orphan processes) — eighteen tools. There is no raw-file write tool; writes are semantic operations only.

**Default MCP mode**: `laplace mcp` discovers the nearest `laplace/schema.yaml` upward from its startup cwd and fixes that vault for the server lifetime. If discovery finds none, startup still succeeds in an empty mode rooted at that cwd: initialization and tool listing remain available, `laplace_vaults` returns no vaults, and vault-dependent calls report that no loadable vault exists. An explicit invalid `--vault DIR` remains a startup error.

**Multi-vault scan mode**: `laplace mcp --scan DIR` discovers every vault under DIR (every directory holding a schema.yaml, gitignore-aware) and serves them all from one process. Every tool takes an optional `vault` selector (schema name or path suffix) — required only while several loadable vaults exist; `laplace_vaults` lists what was found with entity counts and validity, so unloadable vaults are disclosed, never silently skipped.

## 6. Drift (cross-session calibration as a tool)

`laplace drift [--since REV] [--json]` — the session-start freshness audit in one call. Requires git.

- **Base**: last commit touching the vault (`--since` overrides).
- **Changed set**: **git-tracked** paths changed since base, plus dirty working-tree paths, minus the schema's `ignore` globs — and always minus the vault directory itself (map maintenance is not unmapped territory). Operating on tracked paths means `.gitignore` is inherently respected; `ignore` exists for *tracked* noise (lockfiles, vendored reference code).
- **Report**: `stale` — entities whose `source` globs match changed paths (`{ref, paths, commits}`); `uncovered` — changed paths matching no entity's globs (unmapped territory); `unanchored` — count and ratio of entities with no `source`; `dead-anchor` — anchors resolving only to git-untracked/ignored paths (they glob fine on disk, but git-driven drift can never watch them — a distinct failure from matching nothing, which `validate` catches). All four disclose blindness, so silence is never mistaken for cleanliness.
- Per-entity file history additionally gives each entity its own last-touched timestamp for free.
- Exit 0 always (informational). No git or zero anchors → explicit notice, never a silent pass.

Anchoring is capability-bound, not domain-bound: a novel anchors 孙悟空 to `chapters/ch0[1-7].md` exactly as a codebase anchors a module to `src/term/**`.

## 7. Summary projection

`laplace summary [--budget N]` (default 1200 tokens) emits the block a harness injects into agent context:

```
<laplace-map project="西游记·前七回" entities="54" relations="121" updated="2026-08-14">
charter: 杀掉或改写一个角色，哪些章回和伏笔要重看？ / 一件法宝的设定变了，波及谁？
kinds: character(21), artifact(9), location(8), event(12), chapter(4)
relation-types: 师从(12), 持有(9), 出现于(74), 宿敌(3)
character: 孙悟空, 菩提祖师, 玉皇大帝, 太白金星, …+7 more
artifact: 如意金箍棒, 凤翅紫金冠, …
recent:
  2026-08-14 +event:乱蟠桃 ~character:太上老君
  2026-08-13 map bootstrap
This map is authoritative. Details: laplace query / MCP tools; do not guess beyond it.
If your work touches an entity (add/rename/remove/re-relate), update the map in the same turn via add/link/update.
</laplace-map>
```

- **Tiers**: T0 header + charter + kinds + discipline footer → T1 + relation-types → T2 + recent (last 5 vault commits; per-entity `+`/`~`/`-` markers when derivable from paths) → T3 + per-kind name lists. Fit: start at T3, truncate name lists largest-kind-first with `…+N more`, then drop tiers. T0 always emits.
- **Token estimation is CJK-aware**: `ceil(ascii/4) + cjk + ceil(other/2)` (ASCII-calibrated ÷4 undercounts CJK badly — bingo#40's lesson).
- The discipline lines are format contract — the skill and harnesses anchor on their presence.

## 8. The skill (delivery)

One discipline, four focal lengths — the same doctrine folded to the moment it
reaches the agent:

| focal length | carrier | reaches the agent via |
|---|---|---|
| full text | `skill/entity-map/SKILL.md`, **embedded in the binary** (version-locked to the tool behavior it documents) | `laplace skill install` → `<skills-dir>/entity-map/SKILL.md` (the Claude Code / bingo shared convention); `laplace skill show` prints it |
| one paragraph | MCP `initialize.instructions` | injected on connect by clients that honor it — the zero-install channel |
| two lines | the summary's discipline footer (§7) | always in context wherever the summary is injected |
| one line each | MCP tool descriptions | at tool-selection time |

`laplace skill install` auto-detects existing harness directories (`~/.claude/skills`, `~/.config/bingo/skills`, project `./.claude/skills`, `./.bingo/skills`) or takes `--to DIR`; upgrading laplace and re-running install refreshes the text. No marketplace packaging in v1; no harness-specific variants — the discipline is universal, tool semantics live in the MCP descriptions.

## 9. CLI surface

```
laplace init                         scaffold laplace/schema.yaml (+ example entity) — only if absent
laplace add|update|link|unlink|remove|rename   write operations (§2)
laplace schema <op>                  constitutional operations: add-kind/add-relation/set/rename-kind/rename-relation (§2)
laplace validate [--json]
laplace query <tool> …               §5
laplace drift [--since REV]          §6
laplace summary [--budget N]         §7; default budget 1200 tokens (CJK-aware estimation)
laplace skill show|install [--to DIR]  §8
laplace export                       full graph JSON to stdout (same payload as /api/graph)
laplace serve [--port 6174]          read-only HTML view; see §9a
laplace mcp [--scan DIR]             MCP server on stdio (18 tools); --scan serves every vault under DIR
```

Global: `--vault DIR`. Deliberate non-tools: no `fmt` (nothing to format — writes are already canonical, bodies are prose), no `watch` (lazy rebuild), no `stats` (`architecture` is it), no raw-file write (semantic operations only).

### 9a. The view (`laplace serve`)

A scientific-plate projection, structured as overview-first drilldown (Shneiderman: overview → zoom & filter → details on demand), each level fed by an existing query:

- **L0 overview**: title block (vault, counts, updated), charter, FIG. 1 — the **condensation plate** (`architecture`): kinds as circles on a ring (area ∝ entity count, the count printed inside, name lettered beside), one aggregated band per **ordered kind pair** (never one curve per relation type — that is what made dense vaults collapse into label soup), arrowhead toward `to`, width ∝ edge count, a small paper chip carrying the pair's total (the relation name itself when the band carries exactly one type). Hovering a band or node opens the breakdown; beneath the figure a **key** lists every pair's relation split — names live on paper, not fighting on the canvas. A post-mount pass measures each chip and slides overlapping chips along their band until all sit on clear paper. Kind tints are deterministic muted washes (no kind has a reserved color — the palette cycles in declaration order).
- **L1 kind register**: the kind's authoring guide, its connection spectrum, and a ruled list of entities (title · first sentence · tags); multi-select with a bulk copy-refs bar.
- **L2 entry**: prose body as the protagonist (serif, ≤68ch), ref + copy, source anchors, file path; grouped edge lists with notes beside the prose; below, full-width FIG. — the **grouped axial neighborhood** (inbound | center | outbound, grouped by relation type — the group header carries the label once, member lines stay silent; depth 1–2). The figure renders at **natural pixel size, never scaled down**: box width is measured from the labels it must hold (CJK-aware), a side with no edges claims no canvas, and an over-wide figure scrolls rather than shrinks.
- **Plate II — the constellation** (`#/graph`): the Obsidian-style whole-vault view. Every entity is a dot (radius ∝ degree, kind tint), every edge a hairline; hover lights the neighborhood and dims the rest, click enters the entry, nodes drag (reheating the layout), kind chips filter, wheel zooms about the cursor, drag pans. Canvas-rendered for performance: typed-array state, pooled Barnes-Hut quadtree (θ=0.9) for O(n log n) repulsion, d3-flavored springs biased by degree, batched draw calls per kind, viewport culling, screen-space labels under LOD (all labels past a zoom threshold, hub labels always, neighborhood labels on hover). The layout is deterministic (clustered phyllotaxis seed, no RNG) and the loop honors an **idle contract**: once alpha settles and nothing interacts, no frame is scheduled — zero CPU at rest. `#/graph?stress=N` renders a deterministic synthetic graph for performance verification.

Mechanics: GET-only tiny_http, two routes (`/` embedded shell, `/api/graph` = export payload + `ok`/`warnings`), vault reloaded per request, hash-routed SPA (`#/k/<kind>`, `#/e/<ref>`, `#/graph` — shareable, back-button friendly), search-anywhere wormhole, `/` focus · `g` constellation · `Esc` up · `c` copy keys. A broken vault renders its diagnostics — no projections of a broken truth. Zero external dependencies; system font stacks; light plate materials (paper/ink/hairlines/Prussian-blue accent), dark tokens reserved.

## 10. Versioning & compatibility

`apiVersion: laplace/v1` in `schema.yaml` only. Additive evolution within v1; breaking changes bump the major and readers refuse unknown majors with a clear message. Reserved for later definition: per-kind attribute schemas (`kinds.<k>.attributes`), a dedicated `kinds.<k>.guide` field (meanwhile: first sentence = label, rest = guide), body wikilink parsing as implicit "mentions" edges, lifecycle vocabularies, relation inverse display names, namespace facets in `architecture`. Deliberately rejected (recorded so they stay rejected): cardinality constraints (a map is not a database), kind hierarchies (tags cover classification; two mechanisms would fight), inference rules (the engine stays dumb — reasoning is the model's job), tag vocabularies (freedom is the feature), **per-edge propagation overrides** (heterogeneity under the two-question test means the type should split — overrides would dissolve the type-level economy of judgment), **temporal edge models** (the map asserts the present; the vault's git history is the time axis).

## 11. Implementation notes (non-normative)

- Frontmatter: split on `---` fences, parse with the chosen YAML crate (`serde_yaml` is archived — evaluate saphyr-based bridges / `serde_norway` / `serde_yml` at implementation; needs serde derive + error positions).
- Atomic writes: temp file + rename; multi-file transactions (rename op) stage all temps first.
- MCP: hand-rolled stdio JSON-RPC vs official `rmcp` — decide at implementation; the §2/§5 surface is fixed either way.
- Graph: hand-rolled adjacency maps; no petgraph until an algorithm demands it.
- Did-you-mean: Levenshtein within the same kind first, then across kinds; suggest at distance ≤ 2.
- Rust modules: `model` (refs+schema+entity), `vault` (discovery+load+frontmatter), `ops` (write transactions), `validate`, `graph`, `query`, `summary`, `drift`, `serve`, `mcp`, thin `main` (clap).
