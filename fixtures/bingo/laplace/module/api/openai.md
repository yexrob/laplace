---
tags: [net]
relations:
  part-of:
    - subsystem:provider-layer
  implements:
    - contract:provider-client
  consumes:
    - contract:auth-json
source:
    - "src/api/providers/openai.rs"
    - "src/api/providers/presets.rs"
---
OpenAI Responses 协议适配器（POST `{base}/v1/responses`）。映射规则是照着官方 API 参考核过的：
system → instructions（拼接）、messages → input items、tools → function tools
（input_schema → parameters）、thinking → `reasoning.effort`（xhigh/max 收敛到 high）
+ `include:["reasoning.summary_text"]`、max_tokens → max_output_tokens。

最容易踩的一处：Responses 的**两级索引（output_item + content part）要压平成单一 block index**，
被忽略的 item 类型不占位。以及 `output_item.done` 里的权威 arguments 要给空 arguments 兜底。

没有 count_tokens 端点，所以走 contract 的 `Unsupported` 降级到本地估算（D6 的精神）。
`presets.rs` 是内置官方 provider 预设：模板化端点，但**永不模板化模型列表**——
bingo 不过滤模型（D65），要收窄一个订阅提供什么是用户自己在 `providers.<name>.models` 里的事。
