---
tags: [core]
relations:
  part-of:
    - subsystem:hosts
  defines:
    - contract:settings-json
  depends-on:
    - module:storage
    - module:platform
    - module:mcp
    - module:team
    - module:api/client
  consumes:
    - contract:error-codes
    - contract:transcript-jsonl
    - contract:json-events-v1
source:
    - "src/main.rs"
---
CLI 入口与启动链，顺序本身就是设计（D8）：`--version`/`--help` 快路径（不加载重模块）→
解析 home → 存储保留策略清理 → 子命令快路径（share / update 只需要 home，不碰 settings 和 API）→
读三层 settings → 初始化 shell → config lint → MCP 连接 → 分流到 TUI 或 headless。

顶层错误映射也在这里：所有 `?` 冒上来的错误经 `report_error`，
非 TTY（headless / 管道 / CI）走稳定契约 `[error] code=... msg=...`，
json-events 模式下改发 `fatal_event` 并映射退出码。

三层 settings 的合并逻辑其实在 settings.rs，但「什么时候读、读完先 lint 再用」的顺序归这里。
包袱：启动期的 note 在 fullscreen 下会被 alternate screen 抹掉，所以它们同时往 stderr 写一份。
