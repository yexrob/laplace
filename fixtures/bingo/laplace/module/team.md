---
tags: [core]
relations:
  part-of:
    - subsystem:multi-agent
  defines:
    - contract:team-json
  depends-on:
    - module:agents
    - module:channels
    - module:tool/agent
  consumes:
    - contract:agent-md
source:
    - "src/team.rs"
---
项目班子（D31，D54 之后是树）。心智模型：**team 是蓝图**（`.bingo/team.json`，随项目提交），
**room 是工地**（运行时实例 + channel）。三层薄壳，没有引入新运行时：
解析与校验、`spawn_team`/`spawn_tree` 编排（复用既有 Agent spawn 与 ChannelRegistry，
幂等键 = 实例名）、team 记忆。

「启动 ≠ 唤醒」是这里最容易误解的地方：启动只是把成员拉起来待命（零 token、零轮次），
只有 SendMessage 或频道消息到达才真的跑。验收断言就写着「任务到达前 token=0、无轮次日志」。

记忆的键是**项目路径哈希 + 分支**：worktree 场景天然隔离（主仓的 main 与
`.bingo/worktrees/agent-team` 路径和分支都不同），避免绝对路径，
并用 project_hash 断言防止一份拷贝挂到别的机器上。记忆是被**指路**而不是被预加载的（D51）。
