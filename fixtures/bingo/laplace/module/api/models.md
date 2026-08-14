---
tags: [net]
relations:
  part-of:
    - subsystem:provider-layer
  consumes:
    - contract:settings-json
source:
    - "src/api/models.rs"
    - "src/model_families.rs"
    - "src/model_cache.rs"
    - "notes/research/model-catalog-params.md"
---
按模型解析元数据：上下文窗口、thinking 支持、vision 支持。三档，最具体优先（D65/D73）：
用户为这个 provider 声明的 → 家族前缀表 → 保守默认。未知模型回退到 Claude 默认，
保证「不知道更好的信息时」行为与旧版逐字相同。

目录是**值不是全局**：它挂在 Client（provider 权威）上，以 `ModelResolver` 的形式抵达每个测量点，
于是 `/status` 百分比、自动压缩阈值、thinking 门用的是同一把尺子。
固定 200k 常量是被这条设计取代的历史事故——它拿 Claude 的尺子量了每个非 Claude 端点。

`model_families.rs` 是两个所有者的目录文件（`~/.config/bingo/model-catalog.json`）：
`builtin` 段是 bingo 的，每次升级重写（用户的手改会被还原，这正是修正后的默认值抵达用户的方式）；
`overrides` 段是用户的，bingo 永不写。`model_cache.rs` 缓存拉取到的模型列表，
任何失败都降级成「没有缓存」——所以这个模块里没有一个函数返回错误。
