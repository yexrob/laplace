---
tags: [core, danger]
relations:
  binds:
    - module:permission
    - module:tool/bash
    - module:mcp
    - subsystem:tool-layer
    - contract:tool-trait
source:
    - "src/permission.rs"
    - "src/preapproved.rs"
---
**bingo 没有沙箱**（D13 明确记为暂不做）。权限闸 + 模式就是唯一的安全边界，
所以闸门的每一处宽松都是真实的攻击面，而不是体验优化。

由此长出来的细节都不是洁癖：Bash 规则按**子命令**前缀匹配而非整串
（否则 `Bash(ls)` 能放过 `ls; rm -rf ~`，`cd /tmp && rm -rf /` 也能绕过）；
引号未闭合的命令一律不可信，allow 规则不放行；敏感目录的写入必须弹窗，
免疫 bypass 模式；`confirm_reason` 只有 deny 规则能压过。

MCP 工具走同一道闸（`mcp__server` 形式匹配整个 server），因为它们和内置工具共享同一个 Tool trait。
