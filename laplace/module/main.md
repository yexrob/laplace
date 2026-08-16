---
relations:
  defines:
  - contract:cli-surface
  depends-on:
  - module:graph
  - module:drift
  - module:mcp
  - module:ops
  - module:query
  - module:schema_ops
  - module:serve
  - module:skill
  - module:summary
  - module:validate
  - module:vault
  - module:lib
source:
- src/main.rs
---
CLI 壳（clap）：解析命令、分发到各模块、渲染文本结果（JSON 渲染在 query 层）。init 脚手架、render_text 等薄逻辑都在这里，业务判断不进 main。
