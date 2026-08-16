---
tags:
- core
relations:
  consumes:
  - contract:vault-format
  depends-on:
  - module:model
  - module:vault
  part-of:
  - subsystem:query-layer
source:
- src/graph.rs
---
内存属性图，vault 的派生缓存：邻接表 + 对称类型双向物化。只在 validated vault 上构建（校验有错时查询拒绝运行），unresolvable 目标直接跳过。
