---
tags: [core]
relations:
  part-of:
    - subsystem:multi-agent
  defines:
    - contract:agent-md
  depends-on:
    - module:channels
    - module:watch
    - module:query
  consumes:
    - contract:skill-md
source:
    - "src/agents.rs"
---
两件事住在一个文件里，容易混：**定义**（AgentDef，磁盘上的人格模板，frontmatter + system prompt 正文）
和**实例**（AgentRegistry 的条目，一次 spawn 产生的活 session，持有完整消息历史）。

延续一个实例不是重开：把 run_query 返回的完整历史存进条目，
下次 SendMessage 就是「旧历史 + 新指令」再进 run_query，零上下文损失。
状态机 Running / Idle / Stopped；忙时排队（同一条后台任务链在当前轮结束后自动跑下一轮），
闲时带历史唤醒；多条排队指令按序合成一个 prompt。

mailbox 是 `InboxItem::Direct|Channel`，一把锁做「投递 + 认领」的原子操作，所以唤醒不会丢。
`live` 字段持有本轮输出的共享 Arc，工作区打开正在跑的实例时能看到正在生成的文字。
包袱：同步（background:false）子 agent 若整轮被用户打断，可能停在 Running 无驱动状态，
靠 AgentControl stop/delete 清理。
