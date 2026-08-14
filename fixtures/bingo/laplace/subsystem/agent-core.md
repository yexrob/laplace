---
tags: [core]
relations:
  depends-on:
    - subsystem:provider-layer
    - subsystem:tool-layer
    - subsystem:context-management
    - module:permission
  defines:
    - contract:ui-event
  consumes:
    - contract:tool-trait
    - contract:provider-client
    - contract:hooks-json
source:
    - "src/query.rs"
    - "src/query_turn.rs"
    - "src/query_session.rs"
    - "src/tools.rs"
---
一次「回合」的全部机制：把用户输入变成一次流式请求，把模型吐出的 tool_use 变成受控的副作用，
再把 tool_result 塞回历史发起下一轮，直到 end_turn。整个 harness 的立身之本是一句话——
**模型只产生意图，权限、并发、副作用、压缩、记忆、UI 全归本地**（D7）。

四块分工：`query.rs` 是循环本体与横切注入点（权限闸、hook、任务提醒、BM25 召回、图片附件），
`query_session.rs` 持有 Session/Runtime（slash 命令可热改的运行时都挂在 watch channel 上，循环每轮重读），
`query_turn.rs` 管流内重试（D61/D62：一次重试整段重来，失败的尝试在任何内容落历史前被丢弃），
`tools.rs` 按 depth 和实验开关组装工具池。

约定：停止条件不看 stop_reason（不可靠），只看这一轮实际出现的 tool_use 块。
包袱：query.rs 已经越过 130KB，横切逻辑都堆在这里；它同时是 UiHooks 的生产端，
所以任何前端改动最后都要回到这个文件对表。
