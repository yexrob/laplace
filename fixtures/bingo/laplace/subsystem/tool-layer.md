---
tags: [core]
relations:
  defines:
    - contract:tool-trait
  depends-on:
    - module:watch
    - module:tasks
source:
    - "src/tool/**"
    - "src/tools.rs"
---
所有工具的实现区，加上把它们组装成一个池子的那道装配线。工具彼此不认识：
每个只实现 `Tool` trait，能不能跑由权限闸决定，怎么跑由 executor 决定。

装配（`tools.rs`）本身带语义：depth==0 才拿到管理类工具（AskUserQuestion / SendMessage /
AgentControl / Team），depth==1 且有实例名的成员在实验开关下只拿到 Post——
工具池的形状就是 hub-and-spoke 拓扑的编码方式。MCP 工具在同一次装配里并进来，
握手放后台，坏 server 超时后在下一轮 assemble 时报告一次。

约定：默认 fail-closed（不并发安全、非只读）；`confirm_reason` 是唯一能压过 bypass 模式的开关，
只留给「后果必须由用户本人承担」的调用。
