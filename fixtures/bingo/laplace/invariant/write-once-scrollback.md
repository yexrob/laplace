---
tags: [ui]
relations:
  binds:
    - module:tui/term
    - module:tui/chat
    - subsystem:rendering
    - subsystem:image-pipeline
    - subsystem:workspace
source:
    - "src/tui/term.rs"
    - "src/tui/statics.rs"
---
**定稿的行只推进终端自己的 scrollback 一次，此后永不再碰。** 视口以上的任何东西都不重绘。

这是 inline 宿主的立身之本（D26/D27）：这样 resize 时的重排行为和普通 shell 输出完全一致，
用户的选中、复制、搜索都还在。代价是 inline 没有「原地换内容」这回事——
所以工作区和 agent 会话必须开在 alternate screen 上，而不是就地替换 transcript。

D38 在组合层给了它一个形状：`statics.rs` 的 Block 列表 + **前缀单调**的 settled 标记
（标记停在第一个未定稿的块），冻结是惰性的。
违反的症状：滚回去看历史时看到重复或错位的内容，且用户手上没有任何办法恢复。
