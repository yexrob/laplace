---
tags: [core]
relations:
  depends-on:
    - subsystem:agent-core
  consumes:
    - contract:skill-md
source:
    - "src/experience.rs"
    - "src/tool/experience.rs"
    - "src/bm25.rs"
---
跨 session 的项目经验库：agent 把可复用的操作经验（trigger / summary / steps / verify）沉淀成条目，
下次遇到相似情境被召回。设计上的关键取舍是**不许自我晋升**（D40）——
条目的健康度只能由显式记录的 outcome 改写（active / degraded / stale），
模型不能因为「我觉得这条有用」就给自己发奖章。

召回走 BM25（D75），语料是这个项目已提交的经验条目加抽取出来的记忆事实，
每次查询现建索引、不落盘——几十篇文档微秒级打完分，引一个搜索引擎依赖买不到任何东西。
分词自带召回语义：ASCII 词做 4 字符前缀词干，CJK 串切字符二元组（不切的话一整句中文就是一个 token，永远匹配不上）。

只有 active 条目会被无提示地注入——把已知失效的模式主动推给模型，是拿记录去打自己的脸。

存储侧（`experience.rs`）按项目分片（`project_key` 是 cwd 的哈希），条目是 frontmatter + 正文，
复用 skills 的 frontmatter 解析器。工具侧是五个：propose / commit 两段式（提议与入库分开，
用户能在中间截住）、query、outcome、forget。`Forget` 是一等操作而不是删文件——
经验库要能承认自己错了，而承认必须留痕，否则下一个 session 会把同一条错误经验重新提议一遍。
