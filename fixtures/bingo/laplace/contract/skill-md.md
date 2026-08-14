---
tags: [contract, io]
source:
    - "src/skills.rs"
    - "src/skills/bundled/**"
---
`<name>/SKILL.md`：YAML frontmatter（name / description / when_to_use / argument-hint 等）+ markdown 正文。
用户和项目各有一个 skills 目录，内置 skill 编译进二进制。

frontmatter 解析器（`parse_frontmatter_pairs`）是这个仓库里被复用得最广的小东西：
skill、agent 定义、team 记忆条目、experience 条目都吃它——支持任意键和折叠/字面量标量，
但**不是完整 YAML**，只处理单行 `key: value` 那一档。改它会同时波及四个格式，这一点在代码里看不出来。

正文里的 `$ARGUMENTS` / 具名参数由 `substitute_arguments` 替换。
内置 `guide` skill 是 bingo 自己的使用与诊断手册，受 invariant:guide-sync 约束。
