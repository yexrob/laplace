---
tags: [ui]
relations:
  part-of:
    - subsystem:workspace
  depends-on:
    - module:tui/chat
    - module:agents
    - module:channels
source:
    - "src/tui/slack.rs"
---
Slack 皮肤的**纯函数行构造器**——屏幕上每一行都由这里算出来，不碰终端、不持有状态，
所以整套布局可以在没有终端的情况下单测。映射表一对一：workspace = team，
`#channel` = channel，私信 = 子 agent 实例，app 消息 = agent 的文字回复。

宽字符是这里的常驻陷阱：中文名和头像 chip 混排时，背景色块很容易在整行铺设时漏出「洞」，
修法是按显示宽度而不是按字符数铺；chrome 里禁止使用象形字符（不同终端宽度不一致，
一个 emoji 能把整行推歪）。目视校验靠 `slack_preview.rs` 的逐格 `ch` 步进截图法——
浏览器的 CJK 度量不是 2:1，不做逐格步进就会凭空造出终端根本没有的错位。

D48/D49：谁发言决定气泡归属，只有 Post 能进房间；视图不再把运行时的原话直接甩给用户看。
