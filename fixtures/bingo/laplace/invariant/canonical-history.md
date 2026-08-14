---
tags: [core]
relations:
  binds:
    - module:compact
    - contract:transcript-jsonl
    - subsystem:persistence
    - subsystem:context-management
source:
    - "src/compact.rs"
    - "src/transcript.rs"
---
**压缩不重写历史，只追加一行标记；canonical history 永远完整。**（D74）
被压掉的部分只是不再进入下一次请求的投影，而不是从盘上消失。

这样 resume、`bingo share` 的页面、以及事后审计看到的是同一段过去，
而不是三份互相矛盾的删节本。`summary_message` 是字节级契约：投影靠它定位，
所以它的措辞不能随手改；注入摘要必须走 record（落盘 + 进历史），不能直接 push 进内存。

违反的症状是最难查的一类：一次成功的 session 在 resume 之后行为不一致，
而 transcript 本身看起来完全正常。
