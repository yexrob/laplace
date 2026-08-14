---
tags: [contract, io]
source:
    - "src/transcript.rs"
---
每个 session 一个 JSONL 文件（`{slug}-{ts}`），`--continue` / `/resume` / `bingo share` 都从它恢复。
它是这个 harness 里最接近「真相」的东西：历史一旦写下，**只追加、不重写**。

D74 把压缩改成了追加一行标记而不是重写历史——canonical history 保持完整，
压缩只是投影。这样 share 页面、resume、审计三者看到的是同一段过去。

顺序约定：先落盘再进内存历史（`record` 的固定顺序），避免出现「内存里有、盘上没有」的窗口。
保留策略见 subsystem:persistence。
