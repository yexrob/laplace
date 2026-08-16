---
relations:
  consumes:
  - contract:vault-format
  depends-on:
  - module:validate
  - module:vault
source:
- src/drift.rs
---
会话开始的新鲜度审计（SPEC §6）：stale / uncovered / unanchored 三类盲区全披露，git 驱动，永不静默通过。锚点是能力约束而非领域约束——小说锚章回与代码锚模块是同一机制。
