---
tags: [core]
relations:
  part-of:
    - subsystem:context-management
  depends-on:
    - module:api/models
    - module:hooks
  gates:
    - subsystem:provider-layer
  consumes:
    - contract:provider-client
    - contract:transcript-jsonl
source:
    - "src/compact.rs"
---
自动与手动压缩。触发有两条路：`TokenGate` 在请求前按阈值判断（阈值来自当前模型的窗口，
不是固定常量），以及 provider 侧 400 溢出后的补救压缩（D59）——后者是承认「预算算错了」的兜底。

三个常量都有故事：`KEEP_RECENT` 从 8 提到 12，因为工具回合消耗消息很快
（一次 tool_use + 一次 tool_result），8 条可能就是四次工具调用而不含引发它们的对话；
摘要输出预算从 1024 提上来，因为再好的指令也塞不进那么点 token；
回显进摘要提示词的 tool input 有长度上限，够认出命令或路径就行，
否则一次文件写入就占满整个提示词。

两条硬约束：切分点必须安全跨过 tool_result 边界（否则留下孤儿 tool_result，之后每次请求都 400）；
压缩只追加标记行、**不重写 canonical history**（D74）。连续失败三次熔断，`/compact` 可强制。
