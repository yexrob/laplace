---
relations:
  binds:
  - module:vault
  - module:graph
  - module:summary
  - module:serve
  - module:mcp
source:
- docs/SPEC.md
---
vault 是唯一权威工件；graph、summary、view 都是派生、确定、可弃的投影。（SPEC §0.1）谁把派生层当真源，谁就会在重建后看到两个不同的「事实」。
