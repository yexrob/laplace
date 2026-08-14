---
tags: [contract, core]
source:
    - "src/tool/mod.rs"
---
`trait Tool` —— 模型能力的唯一入口形状：name / description / input_schema（schemars 生成，
输入结构体是单一真相）/ call，加上一组供上层判断的属性谓词
（is_concurrency_safe / is_read_only / is_destructive / is_edit_tool / confirm_reason）。

**权限检查刻意不在 trait 里**（D2）：权限是横切的，走统一闸门；工具只声明自己是什么，
不声明自己能不能被用。默认值一律 fail-closed。

内置工具、MCP 工具、子 agent 工具走的是同一个 trait，所以「工具」这个词在整个仓库里只有一种含义。
模型返回的参数由 serde 反序列化校验，错误以 is_error 回喂给模型——不引入 jsonschema 校验器，
九个静态 schema 上它买不到东西。
`confirm_reason` 是唯一能压过 bypass 模式的机制，只有 deny 规则能压过它。
