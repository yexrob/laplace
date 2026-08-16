---
tags:
- core
relations:
  consumes:
  - contract:vault-format
  depends-on:
  - module:model
  - module:vault
  gates:
  - module:ops
  - module:schema_ops
  part-of:
  - subsystem:write-path
source:
- src/validate.rs
---
三层校验（SPEC §3）：结构、声明、引用，dangling 带 did-you-mean。是直编文件与合并后的和解器，也是 CI 门；walk_project_files 与 drift 共享。
