---
tags: [core]
relations:
  part-of:
    - subsystem:multi-agent
  implements:
    - contract:tool-trait
  depends-on:
    - module:agents
    - module:team
    - module:channels
    - module:watch
    - module:query
  consumes:
    - contract:agent-md
source:
    - "src/tool/agent.rs"
---
模型这边的多 agent 之手：`Agent`（spawn）、`SendMessage`（延续或投递）、`AgentControl`（list/stop/delete）。
只在 depth 0 装配——这是 invariant:hub-and-spoke 的执行点。

`SendMessage` 的节奏是被改过的（D60）：入队即派发，运行中的 agent 在工具轮之间吸收邮件，
而不是等到整轮结束。这让「一边跑一边补充指令」变成可能，代价是收信时机不再是单一确定点。

私信是私密车道（D63/D64）：DM 的内容不进 hub 的公共视野，
但 note 必须诚实说明这一轮的文本被哪个界面读走了——早期版本的 note 在这件事上撒了谎，
修法是在措辞层而不是在传输层。用户发出的私信带 `[DM from user]` 标记，hub 发的不带。

这个文件 142KB，是仓库里最大的非 TUI 单文件（chat.rs 更大）。
D53 的规则也在这条路上生效：钉住班子的项目里，班子是默认劳动力，
在旁边 spawn 出来的子 agent 是临时雇佣，永不进编制。
