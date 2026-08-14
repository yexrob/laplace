---
tags: [core, experimental]
relations:
  part-of:
    - subsystem:multi-agent
  depends-on:
    - module:watch
  consumes:
    - contract:settings-json
source:
    - "src/channels.rs"
---
频道引擎（实验开关 `experimental.agentChannels`）。刻意做成纯状态：
成员表 + serial|free + 单调 seq + 全量日志 + 每成员 seen 游标 + 计数。
`post()` 只做三件事——**盖章**（from 来自 session 实例名，模型伪造不了）、
**serial 陈旧检查**（seen < seq 就把增量带回去弹回给发送者）、**预算闸**
（每 agent 每频道 50 条 / 每频道 500 条，超限冻结并发一次 hub 警告）。

设计原则是「能力普遍、选择自主」：投递即唤醒，但**沉默是零成本的吸收态**——
醒来后不调用 Post 就是沉默，不产生任何传播。排序不由引擎管
（round_robin/gather 在讨论里被删掉了：排序是调度与协议，不是传输），
serial 弹回是**工具结果而不是错误**，模型在同一轮里读到增量，自己决定原样重发、改写还是放弃。

`main` 与 `user` 是保留成员名。频道沉默的纪律写在 system block 里，不写在引擎里（D45）。
