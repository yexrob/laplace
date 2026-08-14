---
tags: [core]
relations:
  binds:
    - module:query
    - module:tool/executor
    - module:compact
    - subsystem:agent-core
    - contract:transcript-jsonl
source:
    - "src/query.rs"
    - "src/compact.rs"
---
**一次请求里，每个 tool_use 必须恰好配一个 tool_result。** 缺一个，
之后每一次带着这段历史的请求都会 400，且错误信息不会告诉你是哪一段。

三处必须守：中断时用 `fill_missing_tool_results` 给未答复的块补 is_error 占位；
压缩时切分点必须安全地跨过 tool_result 边界（否则摘要之后留下孤儿 tool_result）；
子 agent 的历史回填同理。

这是本仓库最贵的一类 bug——症状出现在很久以后的某一轮，
而现场（那次中断、那次压缩）早就不在视野里了。
