---
relations:
  consumes:
  - contract:vault-format
  depends-on:
  - module:graph
  - module:query
  - module:validate
  - module:vault
  part-of:
  - subsystem:projection
source:
- src/serve.rs
---
只读 HTML 视图（SPEC §8）：tiny_http，两个路由，每次请求重载 vault——总是新鲜，永无第二个真源。响应构建是纯函数，测试不碰 socket。
