---
tags: [io]
relations:
  defines:
    - contract:transcript-jsonl
  depends-on:
    - module:storage
    - module:tasks
source:
    - "src/storage.rs"
    - "src/transcript.rs"
    - "src/share.rs"
    - "src/share_html.rs"
    - "src/tui/history.rs"
    - "src/tasks.rs"
    - "notes/design/prd-share.md"
    - "notes/acceptance-share.md"
    - "notes/design/share-page-design.md"
    - "notes/design/share-page-template.*"
    - "notes/design/share-review.js"
    - "notes/design/opencode-share-reference.md"
---
所有落盘的东西和它们的保留策略。数据分两处：`~/.local/share/bingo/` 放运行数据
（transcripts / history / shares / tasks），`~/.config/bingo/` 放用户配置与记忆（memdir / agents / model-catalog）。
凭据单独在 auth.json，永远不进项目层设置——项目层 `.bingo/settings.json` 是要提交的。

保留策略是有意收敛的（D11）：transcript 30 天 TTL + 最近 100 个的上限 + 24 小时活跃宽限，
share 快照跟随 transcript 删除，prompt history 同样的 TTL + 100 文件上限。
公开 HTML 导出和任务列表**不在**这套策略里，永不自动删。

约定：写入一律 tmp + rename；share 是增强不是契约，存储失败只警告，绝不阻塞 session。
