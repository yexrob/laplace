---
tags: [net]
relations:
  part-of:
    - subsystem:provider-layer
  depends-on:
    - module:api/models
    - module:api/anthropic
    - module:api/openai
  consumes:
    - contract:provider-client
    - contract:settings-json
    - contract:auth-json
source:
    - "src/api/client.rs"
    - "src/api/providers/mod.rs"
---
provider 门面（D33）：持有 provider 表（`Arc<dyn ProviderClient>`）、当前 provider 开关，
以及 `/provider` 与 `/model` 菜单要显示的信息。协议行为全在适配器里，消费者永远看不到 wire JSON。

`build_provider` 是**唯一**知道「配置怎么映射到适配器」的地方——多一处就多一份要同步的真相。
认证来源有三种：settings 的静态 apiKey（优先）、auth.json 里现读的 StoredKey
（会话中途登录立即生效，不用重启）、OAuth TokenProvider（懒刷新 + single-flight）。

D39 补的一条：顶层 key 缺失时，默认 provider 降级成一个 fail-fast 的 Unconfigured 适配器，
而不是让整个 TUI 起不来——否则用户连「在 TUI 里登录」这条路都走不了。
