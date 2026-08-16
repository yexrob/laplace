---
tags:
- core
relations:
  depends-on:
  - module:model
  - module:validate
  - module:vault
  implements:
  - contract:vault-format
  part-of:
  - subsystem:write-path
source:
- src/ops.rs
---
六写操作（SPEC §2）：add/update/link/unlink/remove/rename。统一事务形状：前置检查 → 内存克隆应用 → 全库重校验（出现新错即中止）→ 原子落盘。remove 拒绝有入边者，dangling 靠构造不可能。
