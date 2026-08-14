---
tags: [core]
relations:
  part-of:
    - subsystem:tool-layer
  depends-on:
    - module:watch
  consumes:
    - contract:tool-trait
source:
    - "src/tool/executor.rs"
---
执行队列（D7）：连续的并发安全工具成批并行（上限 10），非安全工具单独串行，
**永不越过一个未完成的写操作**，结果按插入顺序返回。

取消信号的两条必须品在注释里写得很清楚，因为它们各自对应一个真实事故：
必须先 `borrow_and_update` 清版本号，否则在 `send_replace` 之后订阅的接收者立刻就绪，
select 随机选分支，大约一半的回合会丢弃已发出的请求再重发；
sender drop 之后 `changed()` 会立刻持续返回 Err，必须当成「不再可取消」并 park，
否则调用方会空转重建工作。

并发的边界判定来自 Tool trait 的属性谓词，默认 fail-closed——
一个工具没想清楚自己能不能并发，就当它不能。
