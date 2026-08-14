---
tags: [ui, danger]
relations:
  part-of:
    - subsystem:rendering
  depends-on:
    - module:tui/gfx
source:
    - "src/tui/term.rs"
---
inline 终端驱动，也是**整个 crate 唯一被允许碰光标位置、滚动区和清屏的模块**。
其他所有地方只能渲染进 Buffer，或者把定稿行交给 `insert_history`。

架构对标 `ratatui::Terminal::insert_before`（scrolling-regions 特性）与 codex-rs 的自制终端：
定稿行推进终端自己的 scrollback 一次就永不再碰，视口以上什么都不重绘，
所以 resize 时的重排和普通 shell 输出行为一致。

kitty 图片的定位也必须留在这里（`write_gfx`：DECSC / 按位置 CUP / DECRC，
一次同步更新批、一次 flush），gfx 负载里不许带光标转义。
D68 之后光标是终端自己的真光标，它站的位置不画任何东西。
这个文件是这套 UI 里唯一「危险」的地方，改动前先读 D26/D27/D37 三条。
