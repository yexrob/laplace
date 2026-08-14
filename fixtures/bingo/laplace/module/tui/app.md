---
tags: [ui]
relations:
  part-of:
    - subsystem:hosts
  depends-on:
    - module:tui/term
    - module:tui/chat
    - module:tui/gfx
    - subsystem:rendering
  consumes:
    - contract:ui-event
    - contract:error-codes
source:
    - "src/tui/app.rs"
---
事件循环与帧装配，inline 与 fullscreen 共享同一套自上而下的布局：
transcript（只有活的尾巴，定稿的已经进 scrollback）→ status → tasks → warning → help → 输入区。

铁律：transcript 以下的一切是 chrome，由 `chrome.rs` 声明成元素树，
**行数靠渲染量出来，永不预测**（D38）。历史上每一个装配类渲染 bug——chrome 少算一行、
光标漂一格——根因都是手维护的偏移量成了第二份真相。

`desired_placements` 是图片层的输入：读装配好的帧，为每个完全可见的已加载块产出一个 placement
（instance id = url + doc row 的 24 位哈希），交给 gfx 做 diff 收敛。
resize 必须走 force-redraw（清层、丢传输缓存、重传可见部分），
因为终端在 resize 时会遗忘所有 placement 和图片数据——fullscreen 曾经只标 dirty，
于是图片在 resize 后消失，直到某次 doc row 变化恰好换了 instance id 才重传（D37）。
