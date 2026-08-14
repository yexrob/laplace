---
tags: [contract]
source:
    - "src/error.rs"
---
错误码单一真相（feedback-states.md §4.3 的 C 类退出映射）。三条纪律：
每个模块错误枚举都实现 `ErrorCode`，match **穷举所有变体、不许 `_` 兜底**，
还没有稳定码的变体必须显式返回 `GENERIC`（显式行为，不是隐式回退）；
CLI 日志和 TUI 渲染共用同一个 `map_error`，不许各自映射；
**码值是 semver**——发布后只增不改不复用。

新增一个码 = 新变体映射 + drift-guard 测试里一条新断言，缺一样 CI 就红。
这条纪律是为了让 `[error] code=... msg=...` 这行字对脚本和 CI 长期可依赖。
