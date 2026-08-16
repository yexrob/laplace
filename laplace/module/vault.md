---
tags:
- core
relations:
  depends-on:
  - module:model
  - module:validate
  implements:
  - contract:vault-format
  part-of:
  - subsystem:truth
source:
- src/vault.rs
---
发现与加载：从 cwd 向上找 laplace/schema.yaml，路径即身份（kind/ns/name 由文件路径推导）。加载是 total 的——结构问题变成诊断而非 panic，这样 validate 能一次报全。
