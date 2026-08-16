---
relations:
  defines:
  - contract:skill-format
  part-of:
  - subsystem:projection
source:
- src/skill.rs
---
entity-map SKILL.md 嵌入二进制（include_str!），可打印、可安装到 harness 技能目录。单一来源、与工具行为版本锁死；MCP instructions 与 CLI 安装共用这一份文本。
