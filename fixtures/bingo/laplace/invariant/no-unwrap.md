---
tags: [discipline]
relations:
  binds:
    - subsystem:agent-core
    - subsystem:tool-layer
    - subsystem:rendering
    - subsystem:provider-layer
    - contract:error-codes
source:
    - "AGENTS.md"
    - "scripts/check_discipline.sh"
---
**生产代码里不许有 unwrap / expect，也不许有 unsafe**（AGENTS.md「禁止」两条）。
每一个错误分支都要被处理，错误类型走 thiserror。

守它的不是评审而是 CI：`scripts/check_discipline.sh` 跑
`cargo clippy --bins -D clippy::unwrap_used -D clippy::expect_used`，
同一个脚本还压着 4000 行的单文件行数上限。测试和真正不可达的分支是例外。

违反的症状：一个 panic 会把整个 TUI 连同用户没保存的输入一起带走，
而且 alternate screen 下 panic 消息经常被终端恢复吞掉，现场都拿不到。
