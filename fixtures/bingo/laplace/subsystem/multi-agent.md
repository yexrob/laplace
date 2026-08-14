---
tags: [core]
relations:
  depends-on:
    - subsystem:agent-core
    - module:watch
  consumes:
    - contract:agent-md
    - contract:team-json
source:
    - "src/agents.rs"
    - "src/tool/agent.rs"
    - "src/team.rs"
    - "src/team_cmd.rs"
    - "src/tool/team.rs"
    - "src/channels.rs"
    - "src/tool/channel.rs"
---
bingo 的多 agent 体系，三层叠出来的（D29 → D31 → D43/D54）：
**定义**（`.bingo/agents/*.md` 里的人格模板）→ **实例**（一次 spawn 产生的活 session，带完整历史）→
**编队**（`.bingo/team.json` 声明的项目班子）+ **房间**（channel，成员间的可见消息流）。

控制平面是 hub-and-spoke：只有主 session 拿得到管理工具，子 agent 不管兄弟。
延续一次子 agent 不是重开，而是「旧历史 + 新指令」再进 run_query，零上下文损失。
Channel 引擎只有四条原语（可见性、serial/free 提交检查、投递即唤醒、运行时盖发件人章），
排序冲突交给模型自己重试收敛——引擎不懂场景，纪律全在提示词里。

包袱：三层是分三次长出来的，命名层次并不整齐（team 是蓝图、room 是工地、instance 是活人）；
D54 之后 team 变成树，每次读写都覆盖整棵树。
