---
tags: [ui]
relations:
  depends-on:
    - module:tui/term
    - module:tui/chat
source:
    - "src/tui/el.rs"
    - "src/tui/statics.rs"
    - "src/tui/chrome.rs"
    - "src/tui/view.rs"
    - "src/tui/line.rs"
    - "src/tui/term.rs"
    - "src/tui/markdown.rs"
    - "src/tui/theme.rs"
    - "src/tui/picker.rs"
    - "src/tui/input.rs"
    - "src/tui/keys.rs"
    - "src/tui/math.rs"
    - "src/tui/activities.rs"
    - "src/tui/test_util.rs"
    - "notes/design/picker-model.md"
    - "notes/design/tui-test-infra.md"
---
从「状态」到「终端上的字节」的整条流水线。形状借了 Ink，但**没有借它的 runtime**（D38）：
`el.rs` 是元素树（组件是返回元素的纯函数，一次前序遍历同时产出行、点击区间和光标位置），
`statics.rs` 是 Static 区（transcript 作为 Block 列表，前缀冻结进 scrollback），
`chrome.rs` 声明 transcript 以下的每一段，`view.rs` 把与渲染库无关的行模型翻成 ratatui text，
`term.rs` 是唯一碰终端的地方。

立这一层是为了消灭一类 bug：D26 之后驱动是对的，但视图仍靠手数偏移量拼装——
chrome 少算一行、光标漂一格，根因都是「第二份真相」。所以这里的铁律是
**高度靠渲染量出来，永不预测**；注解不带偏移，绝对位置由遍历算。
明确不要 VDOM、不要 hooks、不要组件保留态（D16/D25 的渲染风暴就死在这上面）。
