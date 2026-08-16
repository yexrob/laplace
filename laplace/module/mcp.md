---
tags:
- core
relations:
  consumes:
  - contract:skill-format
  - contract:vault-format
  defines:
  - contract:mcp-protocol
  depends-on:
  - module:graph
  - module:model
  - module:drift
  - module:ops
  - module:query
  - module:schema_ops
  - module:validate
  - module:vault
  part-of:
  - subsystem:projection
source:
- src/mcp.rs
---
MCP server on stdio（SPEC §5）：newline-delimited JSON-RPC，18 个工具（7 查询 + validate + drift + 6 写 + schema_edit + vaults）。每调用重载 vault：毫秒级全量重建，永远正确胜过聪明缓存。initialize 的 instructions 是 skill 的蒸馏。
