---
relations:
  binds:
  - module:ops
  - module:schema_ops
source:
- docs/SPEC.md
---
每个写操作先对活图校验、后落盘；失败零写入。（SPEC §0.3）通过写路径，vault 永不进入非法态——悬空引用在写时被同步拒绝。
