---
tags: [contract, net]
source:
    - "src/api/contract.rs"
---
provider 中立协议契约（D33）：`NeutralRequest` / `SystemBlock` / `StreamEvent` / `ThinkingLevel` /
`Capabilities` 加上 `ProviderClient` trait（stream → BoxStream、complete_text、list_models、
count_tokens、auth_status）。**消费者永不接触 wire JSON**——主循环、压缩、记忆、TUI 只认这套类型。

这是 AGENTS.md「边界被独立消费就先定契约」规则的样板：契约先立，anthropic 和 openai 两个适配器
对着同一张表验收（同一回合的双协议 fixture 必须产出同一串 StreamEvent）。

`ClientError` 也住在这里，其中 `Unsupported` 是关键设计——openai 没有 count_tokens 端点，
于是降级到本地估算而不是报错；`Config`（未知 protocol）在启动期就以 CONFIG_INVALID 失败。
缓存控制是 anthropic 独有的，thinking 摘要映射到 UI 但不逐字回放。
