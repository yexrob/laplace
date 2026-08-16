---
relations:
  binds:
  - module:model
  - module:validate
  - contract:vault-format
source:
- docs/SPEC.md
---
所有 kind 与关系类型都在 schema.yaml 声明；代码不硬编码任何领域。（SPEC §0.4）通用性靠建模不同领域证明，不靠特判某个类别。
