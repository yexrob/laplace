---
relations:
  consumes:
  - contract:vault-format
  depends-on:
  - module:graph
  - module:model
  part-of:
  - subsystem:query-layer
source:
- src/query.rs
---
七查询（SPEC §5）：search/get/neighbors/trace/impact/architecture/schema。每工具返回 JSON，文本渲染在 main；impact 跑在传播有向图上。
