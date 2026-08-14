---
tags: [io]
relations:
  part-of:
    - subsystem:tool-layer
  defines:
    - contract:skill-md
source:
    - "src/skills.rs"
    - "src/skills/bundled/**"
    - "src/tool/skill.rs"
---
技能加载：内置（编译进二进制）+ 用户层 + 项目层，同名就近覆盖。
一个 skill 是 `<name>/SKILL.md`，frontmatter 给元数据、正文给指令。

`parse_frontmatter_pairs` 是这个模块最被低估的输出：它被 agent 定义、team 记忆条目、
experience 条目共用（任意键 + 折叠/字面量标量，但**不是完整 YAML**）。
四种格式共享一个解析器，改它会同时波及四处——这条耦合在源码里要靠 grep 才看得见。

内置 `guide` skill 是 bingo 自己的使用与诊断手册，受 invariant:guide-sync 约束：
它过期就等于模型对用户撒谎。
