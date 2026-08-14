---
tags: [core]
relations:
  part-of:
    - subsystem:context-management
  depends-on:
    - module:platform
    - module:api/models
    - module:skills
  consumes:
    - contract:skill-md
source:
    - "src/system.rs"
---
系统提示装配：基础段（角色与规则）→ 工具说明 → 记忆层（memdir / 项目 CLAUDE.md 与 AGENTS.md /
`--add-dir`）→ 技能清单 → 会话附加段。

**分段顺序就是缓存策略**（D10）：cache_control 打在 system 与 messages 的尾部断点上，
保证跨轮次只有尾部变化。所以往中间插一段等于让整段缓存失效——
这件事在代码里看不出来，只会表现为账单变贵。

`model_capability_block` 把当前模型的能力（vision / thinking / 有效窗口）写进提示，
让模型知道自己这一轮是什么身体。环境块里报的是**解析后的真实 executor 与 shell dialect**（D71），
因为工具名 `Bash` 是模型看到的最强先验，弱的环境提示压不过它。
