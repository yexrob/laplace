---
tags: [contract, io]
source:
    - "src/team.rs"
    - "src/team_cmd.rs"
---
`.bingo/team.json`（camelCase，随项目提交）：`name` + `channel{mode: serial|free, messageLimit}` +
`members[{name, agent}]`，D54 之后是一棵树。它是**蓝图**，不是运行时状态——
房间和实例是工地，蓝图只说该有谁。

成员引用 AgentDef 而不内联人格：人格的单一真相留在 `.bingo/agents/<name>.md`，
一个人格可以参加多个 team。校验与启动共享同一份解析源：**validate 过了，start 就必须成功**。

错误信息是三段式（文件路径 + 字段路径 + 期望），和 spawn/validate 共用同一套措辞。
配套的 `.bingo/team-norms.md` 是这支班子的书面工作约定，每个成员都带着它（D53）。
