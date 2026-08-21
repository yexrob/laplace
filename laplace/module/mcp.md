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
MCP server on stdio（SPEC §5）：newline-delimited JSON-RPC，18 个工具（7 查询 + validate + drift + 6 写 + schema_edit + vaults + serve）。每调用重载 vault：毫秒级全量重建，永远正确胜过聪明缓存。initialize 的 instructions 是 skill 的蒸馏。

默认启动从 MCP 进程 cwd 向上发现 `laplace/schema.yaml`；若尚未初始化 vault，服务器仍以该 cwd 为根的空模式完成握手并暴露工具，`laplace_vaults` 返回空列表，依赖 vault 的调用给出无可用 vault 错误。显式无效的 `--vault` 仍拒绝启动。
