---
tags: [cross-cutting]
relations:
  part-of:
    - subsystem:agent-core
  consumes:
    - contract:ui-event
source:
    - "src/watch.rs"
---
可观察对象的通知机制：任何「能被盯着」的东西——后台命令、子 agent、频道——
共享同一个状态机（Running → Idle → Done/Failed/Cancelled）。
状态变化广播给 TUI，终止态排队注入主 agent 的上下文。

它是「后台异步」这件事在 bingo 里的统一形状：不是每个功能各自发明一套通知，
而是都变成一个 watchable。`WatchKind` 是契约字段（D29 用它替换了 D28 靠标签前缀区分的做法），
标签必须唯一，否则 TUI 会把两行按标签撞在一起。

注入时机是回合边界，不是立刻——否则通知会插进模型正在进行的推理中间。
