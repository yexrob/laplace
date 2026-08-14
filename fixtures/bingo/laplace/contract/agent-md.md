---
tags: [contract, io]
source:
    - "src/agents.rs"
---
子 agent 的人格定义文件：`~/.config/bingo/agents/*.md` 与 `.bingo/agents/*.md`，
frontmatter 是 `name/description/model/provider/thinking`，正文是它的 system prompt
（替换父 prompt；留空则继承）。项目层同名覆盖用户层，并把 source 标成 Project。

优先级是三段：显式参数 > 定义 > 继承，且 model/provider/thinking 互相独立。
**跨 provider 边界**是踩出来的规矩（#19）：fork 到与父 session 不同的端点时，
父模型和 thinking 等级都不继承——model 必须显式给（缺了就早失败「provider X requires a model」，
而不是让父模型名撞上另一个端点的 model not found），thinking 默认关（DeepSeek/Ollama 端点不吃这个参数）。

定义数量是个位数，所以没有 mtime 缓存；D69 之后每次 start 都重读定义。
