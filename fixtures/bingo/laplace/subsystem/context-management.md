---
tags: [core]
relations:
  depends-on:
    - subsystem:provider-layer
    - module:compact
    - module:system
  consumes:
    - contract:transcript-jsonl
source:
    - "src/compact.rs"
    - "src/budget.rs"
    - "src/system.rs"
    - "src/memory.rs"
    - "src/context_usage.rs"
    - "src/model_families.rs"
    - "src/model_cache.rs"
    - "src/token_rate.rs"
---
决定「模型这一轮看到什么、还能看多少」的所有机制：系统提示装配（分段是缓存策略，
只有尾部随轮次变化）、token 预算（窗口、输出上限、警告与自动压缩阈值）、
压缩（摘要旧消息 + 保留最近若干条）、memdir 自动记忆。

一把尺子原则：窗口、`/status` 百分比、自动压缩阈值、thinking 门都读**同一个** ModelResolver
（用户声明 → 家族前缀表 → 保守默认，D65/D73）。固定 200k 常量是历史事故——
它用 Claude 的尺子量了每一个非 Claude 端点，小窗口模型于是每轮都压缩，请求还照样 400。

包袱：压缩失败连续三次熔断（`/compact` 可手动强制）；非 Anthropic 端点没有 count_tokens，
只能退到本地估算，而 ASCII 校准的 chars/4 对 CJK 严重低估（#40 的教训）。
