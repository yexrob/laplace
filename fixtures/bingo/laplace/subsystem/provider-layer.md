---
tags: [core, net]
relations:
  defines:
    - contract:provider-client
    - contract:auth-json
  depends-on:
    - module:api/client
source:
    - "src/api/**"
    - "notes/design/provider-oauth.md"
    - "notes/research-oauth-cli.md"
---
多 provider 协议抽象（D33）：一侧是中立契约，一侧是各家线格式，消费者永远看不到 wire JSON。
存在的理由是 bingo 要同时吃 Anthropic Messages、OpenAI Responses 和各类兼容端点，
而主循环、压缩、记忆、TUI 都不该知道这件事。

分层：`api::contract` 是中立类型 + `ProviderClient` trait，`api::providers::*` 是适配器，
`api::client::Client` 是门面（持有 provider 表、当前 provider 开关、`/provider` 与 `/model` 要显示的信息），
`api::models` 负责按模型解析上下文窗口与能力，`api::auth` + `auth.rs` 负责 OAuth 与凭据落盘。

历史包袱：anthropic 适配器是从旧 client.rs 原样搬过去的（重试/退避/超时/400 溢出重算/SSE 解析逐字节不动），
所以它内部的结构比 openai 适配器旧一个世代；能力协商是 v1 静态声明，没有运行时协商。
