---
tags: [net]
relations:
  part-of:
    - subsystem:provider-layer
  implements:
    - contract:provider-client
source:
    - "src/api/providers/anthropic.rs"
    - "src/api/sse.rs"
---
Anthropic Messages 协议适配器。它是手写的（D1）：Anthropic 至今没有官方 Rust SDK，
社区实现要么停更要么玩具级。范围包括 SSE 事件解析（`input_json_delta` 累积到
`content_block_stop` 再解析）、stop_reason、429/529 的指数退避重试（尊重 retry-after，2-3 次封顶）、
400 上下文溢出的重算。

**它是从 D33 之前的 client.rs 原样搬过来的**——重试/退避/超时/溢出重算/SSE/错误映射逐字节不动，
先搬到绿（636 → 639）再写新代码。所以它内部的组织方式比 openai 适配器旧一个世代，
读起来不像是照着 contract 写的，因为它本来就是 contract 的原型。

thinking 块必须逐字回传，包括签名；缓存控制是 anthropic 独有的。
