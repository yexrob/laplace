---
tags: [discipline]
relations:
  binds:
    - module:skills
    - contract:settings-json
    - subsystem:hosts
    - subsystem:tool-layer
source:
    - "AGENTS.md"
    - "src/skills/bundled/guide.md"
    - "notes/design/feedback-states.md"
    - "README.md"
    - "README.zh-CN.md"
    - "notes/design/feedback-states-ac.md"
    - "notes/design/feedback-states-presentation.md"
---
**改动触及用户可见行为（配置项 / slash 命令 / 工具 / 错误信息 / 能力表）时，
必须在同一批里更新 `src/skills/bundled/guide.md`。**触及反馈状态（loading / 错误提示 /
toast / 输出格式）时，还要对照 `notes/design/feedback-states.md` 并回填它的变更记录。

理由不是文档洁癖：`guide` 是内置 skill，模型在回答「怎么配」「为什么不工作」时读的就是它。
文档过期在这里等于**模型对用户撒谎**，而且撒得理直气壮。

这是纯人（或 agent）纪律，没有任何机器在守——所以它最容易被违反，
也最值得在改动开始时就先问一句。
