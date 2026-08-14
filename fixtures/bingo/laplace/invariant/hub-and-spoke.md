---
tags: [core]
relations:
  binds:
    - module:agents
    - module:tool/agent
    - module:channels
    - subsystem:multi-agent
    - subsystem:tool-layer
source:
    - "src/tools.rs"
    - "src/agents.rs"
    - "src/tool/agent.rs"
---
**只有主 session（depth==0）拿得到管理类工具；子 agent 不管理兄弟。**
SendMessage / AgentControl / Team / Channel 只在 depth 0 装配，
depth 1 且有实例名的成员在实验开关下只拿到 Post，更深的层级什么都没有（深度上限 3）。

这不是权限考虑而是拓扑考虑（D14/D29）：一旦子 agent 能重启或改写它所属的班子，
就出现了一个把用户同意夹在中间的环。同理，能问用户问题的只有拥有 UI 的那个 session——
子 agent 的回答通道是它的返回值，不是模态框。

违反的症状不是崩溃，是失控：token 花光、班子自我复制、没人知道是谁下的令。
工具池的形状就是这条不变量的编码方式，改 `tools.rs` 的装配条件就是在改它。
