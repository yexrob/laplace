---
tags: [core, hot]
relations:
  part-of:
    - subsystem:agent-core
  depends-on:
    - module:api/client
    - module:permission
    - module:hooks
    - module:compact
    - module:agents
    - module:tasks
    - module:watch
    - subsystem:experience-library
    - module:channels
    - module:tool/executor
    - module:system
  consumes:
    - contract:tool-trait
    - contract:provider-client
    - contract:ui-event
    - contract:transcript-jsonl
source:
    - "src/query.rs"
---
`query_loop` 的家，bingo 的心脏。一轮的形状：装配工具 → 发流式请求 → 收本轮实际出现的 tool_use
（**不看 stop_reason**，它不可靠）→ 逐个过 `gate_tool`（权限闸 + PreToolUse hook + UI 弹窗，
hook 可以改写入参）→ 交给 executor 并发或串行执行 → tool_result 回填 → 再来一轮，直到 end_turn。

它还是一堆横切逻辑的落点，这些在别处看不出来：任务提醒注入（10 轮没碰 Task 且距上次提醒 10 轮）、
BM25 经验/记忆召回（用户刚说的话触发，只注入 active 条目）、图片占位符解释
（`#[image N]` 没配上图时必须说明原因，否则模型会满世界去找那张图）、
tool_result 5 万字符截断、中断时补 is_error 占位结果。

包袱：文件已越过 130KB，`chat.rs` 那条 4000 行红线在这里同样紧张；
它同时是 `UiHooks` 的生产端，所以任何前端改动最后都要回到这个文件对表。
