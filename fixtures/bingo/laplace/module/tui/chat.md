---
tags: [ui, hot]
relations:
  part-of:
    - subsystem:rendering
  depends-on:
    - module:query
    - module:compact
    - subsystem:persistence
  consumes:
    - contract:ui-event
    - contract:transcript-jsonl
    - contract:settings-json
source:
    - "src/tui/chat.rs"
    - "src/tui/chat_tail.rs"
    - "src/tui/slash.rs"
    - "src/tui/model_menu.rs"
    - "src/tui/chat_tests_a.rs"
    - "src/tui/chat_tests_b.rs"
    - "notes/design/slash-ux.md"
    - "notes/design/slash-command-ux.md"
    - "notes/design/slash-ux-docs-patch.md"
---
聊天状态机 + transcript 块构造器：消息、活动（thinking/tool/diff/watch）、折叠分组，
`build_rows` 产出 `statics::Block`，由 `layout` 排成与显示无关的行文档。
事件从 channel 来（`UiEvent` / `AskRequest`），键鼠从 `on_key` / `doc_click` 进。

它是 UI 侧的重心，也是**文件行数纪律的常年欠债户**：`chat_tail.rs` 是为了绕 4000 行上限
从 chat.rs 切出去的尾巴（不持有状态，只是 `impl super::Chat`），
`model_menu.rs` 同理。切分依据是「行数」而不是「职责」，所以边界并不自然——
找一个方法在哪个文件里，靠 grep 比靠推理快。

flush 游标（`flushed_segments` / `tail_start` / `mark_base`）留在 Chat 上，
紧挨着驱动它的状态；文本换行留在 markdown/`wrap_words`——组件管文本排布，树只管结构。
