---
relations:
  depends-on:
  - module:model
  - module:ops
  - module:validate
  - module:vault
  part-of:
  - subsystem:write-path
source:
- src/schema_ops.rs
---
宪法操作（SPEC §2）：add-kind/add-relation/set/rename-kind/rename-relation。解析 → 变更 → canonical 重渲 → 模拟 → 落盘；rename 原子重写全库引用并移动 kind 目录。
