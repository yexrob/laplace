---
tags: [net]
relations:
  part-of:
    - subsystem:tool-layer
  implements:
    - contract:tool-trait
  consumes:
    - contract:settings-json
source:
    - "src/mcp.rs"
---
MCP 管理器（rmcp 官方 SDK，D3/D24）：stdio（子进程）与 streamable HTTP 两种传输，
连接缓存 + `/mcp` 命令查看状态。远端工具适配成**同一个 Tool trait**，
所以它们和内置工具走同一道权限闸、同一套并发规则。

握手放后台：一次装配不等 server 握手完成，坏 server 超时后落进 failures，
在下一轮 assemble 时报告一次（`drain_unreported_failures`）。
子 agent 共享同一个 manager，但它们的 on_warning 是空操作——
所以只有 depth 0 才能 drain，否则失败报告会被子 agent 吃掉，用户永远看不到。

工具名要归一化（`normalize_mcp_name`），因为权限规则 `mcp__server` 形式要对得上。
