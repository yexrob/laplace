---
source:
- skill/entity-map/SKILL.md
---
SKILL.md 实体地图纪律：仅在用户要求创建或使用实体地图时，首次接入无 vault 的项目才先初始化；支持子 agent 的 harness 由多个只读 scout 按架构、契约、领域、测试与运维分区深扫，主 agent 作为 single writer 统一设计 schema，并通过 CLI/MCP 创建有来源锚点、关系充分且通过审计的实体地图。日常维护继续遵循查询不猜、同轮更新。二进制嵌入版本锁死，skill install 分发。
