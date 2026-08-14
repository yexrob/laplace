# Laplace

> *An intellect which at a certain moment would know all forces that set nature in motion, and all positions of all items of which nature is composed — for such an intellect nothing would be uncertain, and the future just like the past would be present before its eyes.*
> — Pierre-Simon Laplace, on the demon later named after him

**Laplace is that intellect, for your project.**

Any project — a codebase, a manuscript, a research corpus, an operation — is a system of entities and relationships. Laplace maintains the authoritative map of what exists and how things connect, as a single versioned YAML file that AI agents write and humans view.

- **One truth file.** `laplace.yaml` — a Backstage-style descriptor with *zero built-in kinds*: every project defines its own vocabulary (`module`/`service` for code, `character`/`timeline` for a novel, anything for anything). Everything else — graph, queries, summaries, views — is a deterministic projection of this file.
- **The AI maintains, the human views.** Agents keep the map fresh as part of their working discipline; humans browse a read-only view and hand entity references back to the agent. Maintenance is the AI's job, not a human chore.
- **A graph, not a document.** The YAML parses into an in-memory property graph. Structured queries (`search`, `trace`, `impact`, `architecture`) give agents the full picture at a fraction of the token cost of re-reading everything.
- **Harness-agnostic.** A CLI and an MCP server. Works with any agent harness — Claude Code, or your own.

## Status

Early design. See [DESIGN.md](DESIGN.md).

## License

MIT
