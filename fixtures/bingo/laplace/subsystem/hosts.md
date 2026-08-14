---
tags: [core, ui]
relations:
  defines:
    - contract:json-events-v1
    - contract:error-codes
  depends-on:
    - subsystem:agent-core
    - subsystem:rendering
    - module:main
  consumes:
    - contract:ui-event
    - contract:settings-json
source:
    - "src/main.rs"
    - "src/tui/mod.rs"
    - "src/tui/app.rs"
    - "src/json_events.rs"
    - "src/ui.rs"
    - "tests/cli_black_box.rs"
---
同一个 agent 核心的四种驱动方式，各自有各自的对外契约：
**fullscreen**（默认，alternate screen 画布）、**inline**（`--inline`，写一次就进终端 scrollback，
kitty 图片走这条路）、**headless**（`-p/--print`，stdout 是纯回复）、
**json-events**（`--json-events`，stdin 收命令 stdout 吐事件，给 GUI 用）。

模式在 CLI 边界一次解析完，之后传给 `run_tui_session`，两个渲染宿主各自保持自己的行为和测试（D36）。
黑盒契约是真契约：非 TTY 下错误必须是一行 `[error] code=... msg=...`，
stdout 干净、无 ANSI、退出码稳定，`tests/cli_black_box.rs` 跑真二进制来钉住它。

包袱：inline 与 fullscreen 是两套帧装配路径，resize 的恢复策略也不同
（inline 走 clear + rehydrate 的既有管线，fullscreen 只置 force_redraw）；
每加一个 UI 状态都要在两边各对一次。
