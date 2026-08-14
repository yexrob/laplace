---
tags: [contract, io]
source:
    - "src/json_events.rs"
    - "notes/gui-json-events-legacy-check.md"
---
跨进程 GUI 协议 v1：stdin 一行一个 `ClientCommand`（turn.start / turn.cancel / …），
stdout 一行一个事件，`protocolVersion: 1` 显式携带。给的是「另一个程序驱动 bingo」的能力，
不是「另一个终端 UI」。

尺寸上限是协议的一部分而非实现细节：命令行 1MB、事件行 8MB、prompt 100 万字符、
响应 10 万字符、重命名 80 字符——因为对面是别人的进程，无界就是拒绝服务。
`--probe` 让客户端在不开 session 的情况下问出协议版本。

致命错误也走协议：顶层错误在 json-events 模式下发 `fatal_event` 并映射退出码，
不再走人类可读的 report_error 路径。事件语义与 TUI 共享 `UiEvent`，
所以协议演进的第一问永远是「这是新事件，还是既有事件的新编码」。
