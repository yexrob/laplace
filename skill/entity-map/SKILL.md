---
name: entity-map
description: Maintain, initialize, and consult the project's Laplace entity map (a laplace/ vault). Use when a vault exists or the user asks to create or use a map; when answering "what exists here / how are these connected / what does changing X touch"; and whenever your edits add, rename, remove, or re-relate an entity — the map updates in the same turn, never later.
---

<!-- laplace-skill, ships with the laplace binary; reinstall via `laplace skill install` after upgrades -->

# The entity map

The vault (`laplace/` — one markdown file per entity plus `schema.yaml`) is the
project's authoritative picture of **what exists and how things connect**. It is
AI-maintained: you are its writer. Humans read projections; you keep it true.

Two standing rules, above everything else:

1. **Query, don't guess.** The injected summary is a table of contents, not the
   truth. Anything beyond it comes from the tools, never from memory.
2. **Same-turn updates.** If your work adds, renames, removes, or re-relates an
   entity, update the map in the very turn the change lands. A map that waits is
   a map that lies.

Most workflows below are available through the CLI (`laplace …`) and
corresponding MCP tools (`laplace_…`); bootstrap exceptions are called out.
When several vaults are loaded, `laplace_vaults` lists them and semantic MCP
tools accept a `vault` selector.

## First use: initialize before consulting

When initialization was requested, first choose the project root deliberately
(from workspace manifests and project instructions), then reconcile the valid
vault locations: an explicitly selected vault, `<project-root>/laplace/schema.yaml`,
or `<project-root>/schema.yaml` when the project root is itself a vault. An
ancestor vault counts only when its declared root and charter actually cover
the chosen project; do not silently reuse it for a nested project. If no
matching vault exists, build one before treating Laplace as project context.
Initialization is a **survey → constitution → write → audit** pass, not a token
example vault.

### 1. Survey the whole project

Read the project rules and top-level structure first, then inventory every
long-lived area that can answer the charter: source roots, packages/modules,
public or cross-process contracts, schemas and persisted formats, entrypoints,
tests, operations/deployment files, design decisions, and invariants. Follow
imports, manifests, registrations, and references far enough to recover real
relationships; do not infer an edge from names alone.

If the harness supports subagents, dispatch several **scouts** in parallel,
partitioned by concern or source root rather than asking each to scan the whole
repository. A useful code-project split is:

- architecture and runtime flow;
- public interfaces, protocols, schemas, and persistence formats;
- domain concepts and cross-module relationships;
- tests, operations, configuration, rules, and documented invariants.

Give every scout the same evidence format: proposed durable entity identity,
why it outlives a session, a summary-worthy first sentence, exact `source`
globs, proposed relationships with evidence, and uncertainties. Scouts propose
concepts and verbs in project language, not final kinds, relation types, or
refs; the coordinator assigns those after the charter defines the vocabulary.
Require each scout to report both its covered paths and material paths it
deliberately excluded.
**Scouts are read-only**: they must not run `laplace init`, edit the vault, or
call MCP/CLI write operations. Wait for every scout report before designing the
schema. This keeps schema design and disk mutation under a **single writer** and
prevents incompatible kinds, duplicate entities, and lost updates. If subagents
are unavailable, perform the same partitions serially and retain the same
evidence format.

The survey is complete only when every material top-level area is covered by a
scout report or an explicit exclusion and cross-cutting evidence has been
reconciled.

### 2. Design one constitution

The coordinating agent merges the reports, resolves duplicate names and
conflicting edges, and then writes 3–5 charter questions. Derive a small
project-native vocabulary from those questions (normally no more than 7±2
kinds and relation types); never let scouts invent separate ontologies. For
every relation, state `A rel B` in reading order, apply the two-question
propagation test below, and define endpoint constraints when the evidence
supports them. Apply the entity admission test before touching disk. If the
survey yields no real entities or no meaningful relationships, report that the
project is not yet worth mapping and stop without creating a vault.

If a matching vault already exists, skip bootstrap and bind every later
operation to it: pass `--vault <vault-dir>` on CLI calls, and `vault` on MCP
calls when selection is required. A vault located at the project root must
declare `root: .`; the conventional `<project-root>/laplace` vault normally
declares `root: ..`. Current `drift` cannot audit a project-root vault reliably,
so disclose that blind spot and use Git plus the territory ledger there. The
examples below omit the repeated selector only for readability.

When no matching vault exists, initialize only the conventional vault. From the
chosen project root, prepare the complete reviewed `schema.yaml` in a temporary
file first. Then run `laplace init --name project` to create the
conventional `./laplace` vault, using the safe literal placeholder name, and
immediately replace the scaffold schema with the prepared file. Treat these as
one uninterrupted bootstrap step; if replacement or immediate validation
fails, remove only the scaffold created by this step so no valid-looking
placeholder vault remains. Do not run init from a nested working directory: it
creates `./laplace`, not a vault at the discovered project root. If
`<project-root>/laplace/` already contains files but has no schema, stop and
reconcile that state instead of scaffolding into it. If the CLI or direct file
write needed for bootstrap is unavailable, report the blocker; MCP alone cannot
initialize a vault today. Because the current schema operations cannot remove the
scaffold placeholders or set the whole charter/ignore/exclusions surface, the
coordinator must directly replace the new `schema.yaml` once with the reviewed
constitution; do this before any entity exists, then immediately run `laplace
validate` and `laplace query schema`. Do not leave the example `thing` /
`depends-on` declarations unless the survey independently justified them. MCP
has no init tool, so bootstrap requires this CLI step; after it, use
`laplace_schema_edit` for supported constitutional changes and MCP or CLI
semantic operations for entities.

### 3. Populate through one write path

The coordinator is the only writer. Populate in two passes because a relation
target must already exist: first search and add every accepted entity without
edges; then link confirmed relationships after all targets resolve. Use
`laplace query search` / `laplace_search`, `laplace add` / `laplace_add`, and
`laplace link` / `laplace_link`, reading every operation result and link echo.
Prefer one structured `laplace add --stdin` payload when an entity has several
known fields. Add `source` globs that point to real project files so drift can
observe coverage. Validate after each coherent batch so errors stay local.

Be rich in **intent and connectivity**, not inventory noise: include every
long-lived entity needed to answer the charter, with useful bodies and grounded
edges, but exclude leaf symbols, generated files, and facts recoverable by a
cheap file listing. Never create an entity only to make counts look complete,
and never use one catch-all entity or a repository-wide `source` glob to
manufacture coverage. Every accepted entity must support at least one charter
question; every orphan warning needs an explicit reason or a real relationship.

### 4. Audit before use

Run `laplace validate`, fix every error, and review every warning. Hidden source
paths such as `.github/**` are currently skipped by the source walker; record
that blindness instead of adding anchors Laplace will report as dead. Then run
`laplace drift` to disclose current Git-visible stale and uncovered paths; it is
not proof of total repository coverage. Initialization is complete only when
there are zero validation errors, every source anchor resolves, the survey's
territory ledger accounts for every material top-level area as anchored,
path-shaped `ignore`, concept-shaped `exclusions`, or charter-irrelevant. The
territory ledger and `exclusions` are human-reviewed evidence, not drift output.
Representative queries that the map's actual size permits must return useful
answers: `search`/`get` for one entity, `trace` for two connected entities, and
`impact` where a propagating edge exists. The final vault diff must be reviewed. Keep the
vault change with the project change it describes when the user's workflow
calls for a commit; do not commit merely because initialization ran.

## Session start: audit before trusting

Run `laplace drift` (or `laplace_drift`) once before leaning on the map:

- **stale** — entities whose anchored files changed since the map's last commit:
  re-read those entities' territory before trusting their descriptions.
- **uncovered** — changed paths no entity claims: candidate new territory.
- **unanchored / dead-anchor** — disclosed blindness; the audit cannot see there.

Stale entities you rely on this session get fixed first, not noted for later.

## Consulting: match the tool to the question's shape

| the question | the tool |
|---|---|
| "what's the ref for X?" | `search` — always, before adding or guessing a ref |
| "what is X, exactly?" | `get` — frontmatter, prose, edges both ways, file path |
| "what surrounds X?" | `neighbors` (1–2 hops) |
| "how are X and Y connected?" | `trace` — annotated shortest paths |
| "what does changing X touch?" | `impact` — distance-bucketed candidate set |
| "what does this project look like?" | `architecture` — kind-level condensation |
| "what vocabulary does this map speak?" | `schema` — charter, kinds, relation directions |

Reading `impact` correctly: it is **sound with respect to the map and complete
with respect to nothing** — a review list, not an oracle. Distance is the whole
signal: distance 1 is a strong claim, distance 3 an echo. Container questions
("改写第三回要重看什么?") are `neighbors` questions, not `impact` — a pure-sink
kind legitimately has an empty impact closure.

## Maintaining: the write loop

Creation and linking go through the operations — never hand-edit frontmatter
when an op exists (ops validate before disk; hand edits need `laplace validate`
afterwards, and any edit that survives it is legal).

1. `schema` — learn the vocabulary first. Kind descriptions double as authoring
   guides; relation descriptions state who is A and who is B. Never invent a
   kind or relation type inline.
2. `search` the name — the entity may already exist under a variant.
3. `add` / `link` / `update` / `rename` / `remove`. Then **read the echo**:
   `link` prints the source's full edge list of that type
   (`now 师从: [菩提祖师, 唐僧]`) — that line exists to catch slips at the
   moment they happen. `remove` refusing with an inbound list is the map
   protecting itself; unlink first.
4. Write bodies that say what the source cannot: why it exists, what it
   promises, where the history is buried. Never paraphrase file listings — that
   is grep's job. The first sentence becomes the one-line summary everywhere,
   so make it summary-worthy.
5. **The map asserts the present.** 敖广曾持有金箍棒 is prose in the body, not
   a `持有` edge; the vault's git history is the time axis.
6. Commit the map edit **with** the work it describes — one commit, one review.

## Drafting reference: what belongs in the map

Let the project name itself. Its rule documents (AGENTS.md, CONTRIBUTING,
design docs) often supply better kinds such as `contract` and `invariant` than
generic words such as `component`. Ground dependency-like edges mechanically
with imports, manifests, registrations, and references; use judgment for
semantic edges such as `binds` and `gates`.

A charter is 3–5 questions the map must answer. Everything below derives from
those questions — a map is an index of questions, not a model of the world.
For every relation run the two-question test on `A rel B`: *B changed — must A
be revisited?* (yes → `to-source`); *A changed — must B be revisited?* (yes →
`to-target`); both → `both`, neither → `none`. Write that reading direction
into the description. If one type's edges split on the test, the name is
wearing two verbs: split the type.

An entity belongs only when all three are true: something refers to it, it
outlives a session, and changing it would force something else to be revisited.
Bodies explain intent, promises, and buried constraints rather than paraphrasing
file listings. Anchor each entity to the smallest honest set of source globs.

## Constitutional changes

The schema is the constitution; entity edits are daily law, schema edits are
amendments. Adding a kind or relation type must cite the charter question it
serves — if none fits, add the question first or don't add the word. Renames of
kinds and relation types go through `laplace schema rename-*` or the
corresponding `laplace_schema_edit` operation: they rewrite every usage
vault-wide. Prefer a coarse type plus an edge `note` over a family of fine
types.

## What not to do

- Don't enumerate for completeness — every entity is a curation decision.
- Don't duplicate body prose into edges, or edges into prose.
- Don't model the past as present-tense edges.
- Don't downgrade an honest `both` propagation to make `impact` output smaller;
  rely on depth and distance instead.
- Don't leave a rename's reported prose mentions unreviewed — bodies are never
  rewritten mechanically; they are yours to reconcile.
