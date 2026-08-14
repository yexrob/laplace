---
tags: [contract, ui]
source:
    - "src/ui.rs"
---
agent 核心与**任何**前端之间的渲染无关契约：`UiEvent`、各类对话框传输类型，
以及把 `UiHooks` 回调转成 channel 流量的 `tui_hooks` 适配器。
这个模块里不许出现任何终端库的依赖——TUI、GUI（json-events）、测试 harness 消费的是同一份东西。

它存在的意义在 D-gui-json-events 之后才完全兑现：JSON 协议宿主不是第二套渲染逻辑，
而是同一批事件的另一种编码。所以新增一种 UI 反馈时，正确的做法是加 UiEvent 变体，
而不是在 TUI 里私接一条路。

生产端在 `query.rs`（UiHooks 是它的字段），消费端在 `tui/app.rs` 与 `json_events.rs`。
