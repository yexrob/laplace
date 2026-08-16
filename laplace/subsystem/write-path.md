---
source:
- DESIGN.md
---
写路径：ops 与 schema_ops 的事务性写，validate 把关。外部世界（人直编、合并）绕过它，但绕过即失去写前校验，靠 validate 事后和解。
