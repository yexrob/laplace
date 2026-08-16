---
tags:
- core
relations:
  defines:
  - contract:vault-format
  part-of:
  - subsystem:truth
source:
- src/model.rs
---
核心类型：EntityRef、Schema、Entity、FrontMatter、RelEntry、Propagation。SPEC §1 的模型全部落在这一处；ref 的 NFC 归一化在这里，Unicode 原生从类型层保证。
