---
tags: [io]
relations:
  part-of:
    - subsystem:persistence
  consumes:
    - contract:transcript-jsonl
source:
    - "src/storage.rs"
---
home 解析与目录布局的单一出处：`~/.local/share/bingo/{transcripts,history,shares,tasks}`，
Windows 下回退 USERPROFILE。别处一律不许自己拼这些路径。

保留策略也在这里（D11）：transcript 30 天 TTL + 最近 100 个上限 + 24 小时活跃宽限，
share 快照跟随 transcript 删除，history 同 TTL + 100 文件上限。
启动时跑一次、`/gc` 手动跑一次。**公开 HTML 导出和任务列表不在这套策略里，永不自动删**——
那是用户的产出，不是缓存。

清理失败只警告不阻塞启动：清不掉盘上的旧文件，不该成为用不了 bingo 的理由。
