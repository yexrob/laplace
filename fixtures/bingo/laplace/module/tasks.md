---
tags: [io]
relations:
  part-of:
    - subsystem:persistence
  depends-on:
    - module:storage
  consumes:
    - contract:hooks-json
source:
    - "src/tasks.rs"
    - "src/tool/task.rs"
---
任务存储（Task 工具族，D15 的 v2 语义）：`~/.local/share/bingo/tasks/<listId>/<taskId>.json`，
一任务一文件，**跨 session 持久**，数字 id 递增（max+1），读取时逐条容错解析。

关键取舍是**增量补丁语义**而不是 v1 的整表覆盖——并发下整表覆盖是 lost update 的温床。
状态机 pending → in_progress → completed，`deleted` 是永久移除态并顺带清理别处对它的引用。

还有一层「输入修复」：模型经常把 title/name 写成 subject、content 写成 description、
active_form 写成 activeForm，或者把 task 包一层——这些近似错误直接修好，
彻底用错（传 tasks 数组、传 Agent 的参数）则返回指导文本。
这层存在的理由是：一个格式错误让模型重试一轮的代价，远高于在这里认几个别名。
