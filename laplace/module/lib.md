---
relations:
  depends-on:
  - module:drift
  - module:graph
  - module:mcp
  - module:model
  - module:ops
  - module:query
  - module:schema_ops
  - module:serve
  - module:skill
  - module:summary
  - module:validate
  - module:vault
  - module:main
source:
- src/lib.rs
---
库面：导出全部模块，被 main 与集成测试消费。不承载逻辑，只声明公开面。
