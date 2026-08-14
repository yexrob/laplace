---
tags: [discipline]
relations:
  binds:
    - contract:tool-trait
    - contract:provider-client
    - contract:ui-event
    - contract:json-events-v1
    - contract:hooks-json
    - contract:error-codes
source:
    - "AGENTS.md"
    - "notes/research.md"
---
**边界被独立消费时（公共 API、跨进程协议、持久化格式），先以原生形式定契约
（trait / serde schema），各实现共同对表；内部重构不立契约。**（AGENTS.md 架构规则）

D33 是样板：先定 settings v2 + `api::contract` 中立类型 + auth.json 形状，
然后 anthropic 适配器**逐字节搬迁**（基线 636 绿 → 搬完 639 绿之后新代码才开始写），
最后用同一回合的双协议 fixture 断言两个适配器产出同一串 StreamEvent。

反面同样重要：不要给内部重构立契约。多一层 trait 就多一处要同步的真相，
而这条不变量的目的正好是**减少**真相的份数。
