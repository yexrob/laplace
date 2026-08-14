---
tags: [ui]
relations:
  depends-on:
    - subsystem:rendering
    - subsystem:multi-agent
    - module:tui/slack
  part-of:
    - subsystem:rendering
source:
    - "src/tui/slack.rs"
    - "src/tui/entity.rs"
    - "src/tui/avatar.rs"
    - "src/tui/slack_preview.rs"
---
Slack 形状的工作区（ctrl+G 打开）：channel 是频道、子 agent 实例是私信、agent 回复是 app 消息。
这个映射是一对一的——运行时早就有这些东西，换皮不需要发明新概念（D43）。

D47 把它剥光了：没有 rail、没有 sidebar、不画自己的底色（终端自己的背景透上来），
导航交给 ctrl+K 切换器和 alt+↑/↓——一列会话不值它占的那几列宽度。
`slack.rs` 是纯函数行构造器，`entity.rs` 是持有终端的宿主循环（跑在 alternate screen 上，
因为 write-once scrollback 决定了 inline 没有「原地换内容」这回事）。

`slack_preview.rs` 是目视审查用的：把帧渲染成每个终端格一个 `<i>` 且显式 `ch` 步进的 HTML，
浏览器截图才等于终端真实网格——浏览器的 CJK 度量不是 2:1，不做逐格步进就会凭空造出错位。
