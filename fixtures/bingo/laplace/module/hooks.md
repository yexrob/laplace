---
tags: [io]
relations:
  part-of:
    - subsystem:agent-core
  defines:
    - contract:hooks-json
  depends-on:
    - module:platform
  gates:
    - subsystem:tool-layer
    - module:compact
  consumes:
    - contract:settings-json
source:
    - "src/hooks.rs"
---
shell hook 的执行侧：按事件和 matcher 找到用户配置的命令，用平台 shell 起进程，
stdin 喂 JSON、stdout 收决策，超时兜底。

它是策略层而非通知层：PreToolUse 能改写工具输入或直接否决，
TaskCompleted 的 blockingError 能把「已完成」打回去。
所以 hook 的失败模式必须是安全的——超时、非零退出、脏输出都不能变成「默默放行」。

SessionEnd 只给 1.5 秒快速拆卸：退出路径上等一个坏脚本，用户感受到的就是 bingo 卡死。
