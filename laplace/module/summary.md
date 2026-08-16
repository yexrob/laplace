---
relations:
  consumes:
  - contract:vault-format
  defines:
  - contract:summary-format
  depends-on:
  - module:model
  - module:vault
  part-of:
  - subsystem:projection
source:
- src/summary.rs
---
摘要投影（SPEC §7）：harness 注入上下文的 <laplace-map> 块。分层 T0-T3 适配 token 预算；token 估计是 CJK 感知的（ASCII÷4 会严重低估中文——bingo#40 的教训）。
