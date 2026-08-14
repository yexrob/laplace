---
tags: [ui, danger]
relations:
  binds:
    - module:tui/term
    - module:tui/app
    - module:tui/gfx
    - subsystem:rendering
    - subsystem:workspace
    - subsystem:image-pipeline
source:
    - "src/tui/term.rs"
---
**整个 crate 里只有 `term.rs` 被允许碰光标位置、滚动区和清屏。**
其他所有模块只能渲染进 `Buffer`，或者把定稿行交给 `InlineTerm::insert_history`。

这条垄断是 D26 重写买来的：iocraft 时期渲染风暴、diff 残影、resize 越界 panic
（D19/D20e/D20f/D20i）的共同根因，就是危险操作散落在多处、各自维护一份「屏幕现在长什么样」的想象。

kitty 图片是最容易破戒的地方：定位必须留在 `term::write_gfx`
（DECSC / 按位置 CUP / DECRC 的一次同步更新批），gfx 负载里不许带任何光标转义（D37）。
违反的症状不是编译错，而是别人的重绘把你的输出擦掉，或者滚动之后一切错位一行。
