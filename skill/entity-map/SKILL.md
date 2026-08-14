---
name: entity-map
description: Maintain and consult the project's Laplace entity map (a laplace/ vault). Use when a vault exists or the user asks for one; when answering "what exists here / how are these connected / what does changing X touch"; and whenever your edits add, rename, remove, or re-relate an entity — the map updates in the same turn, never later.
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

Everything below runs over the CLI (`laplace …`) or the identical MCP tools
(`laplace_…`). When several vaults are loaded, `laplace_vaults` lists them and
every tool takes a `vault` selector.

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

## Creating a map from scratch (the draft discipline)

Order matters more than effort. The productive sequence, hard-won:

1. **The project's own rule documents first** (AGENTS.md, CONTRIBUTING, design
   docs) — they often hand you the vocabulary verbatim. Let the project name
   its own kinds: `contract` and `invariant` lifted from a repo's AGENTS.md
   beat generic `component`/`layer` every time.
2. **Cheap mechanical sweeps second**: doc-comment headers (`grep '^//!'`),
   decision-record indexes, an import/dependency count. Minutes of grep,
   most of the intent layer.
3. **Charter third**: write the 3–5 questions this map must answer. Everything
   below derives from them — a map is an index of questions, not a model of the
   world. Completeness is a disease, sparsity is the signal.
4. **Vocabulary fourth**, derived from the charter. Budget ≤7±2 kinds and
   relation types. For every relation run the two-question test on `A rel B`:
   *B changed — must A be revisited?* (yes → `to-source`); *A changed — must B
   be revisited?* (yes → `to-target`); both → `both`, neither → `none`.
   Worked example: 孙悟空 师从 菩提祖师 — rewrite the master and the disciple's
   art loses its origin (`to-source`); rewrite the disciple and the master
   stands unchanged. Even designers get directions wrong unaided — write the
   reading direction into every description. If one type's edges split on the
   test, that name is wearing two verbs: split the type.
5. **Entities last.** Admission test, all three: something refers to it; it
   outlives a session; its change would force revisiting something else.
   Ground dependency-like edges mechanically (a grep is honest), semantic edges
   (`binds`, `gates`) by judgment — the mix is what makes the map both true and
   useful. Anchor entities to their files with `source` globs so drift can
   watch them.
6. `validate` to zero errors, read the warnings, commit.

## Constitutional changes

The schema is the constitution; entity edits are daily law, schema edits are
amendments. Adding a kind or relation type must cite the charter question it
serves — if none fits, add the question first or don't add the word. Renames of
kinds and relation types go through `laplace schema rename-*` only: they
rewrite every usage vault-wide atomically. Prefer a coarse type plus an edge
`note` over a family of fine types.

## What not to do

- Don't enumerate for completeness — every entity is a curation decision.
- Don't duplicate body prose into edges, or edges into prose.
- Don't model the past as present-tense edges.
- Don't downgrade an honest `both` propagation to make `impact` output smaller;
  rely on depth and distance instead.
- Don't leave a rename's reported prose mentions unreviewed — bodies are never
  rewritten mechanically; they are yours to reconcile.
