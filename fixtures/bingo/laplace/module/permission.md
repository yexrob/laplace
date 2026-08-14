---
tags: [core, danger]
relations:
  part-of:
    - subsystem:agent-core
  gates:
    - subsystem:tool-layer
    - module:tool/bash
    - module:tool/agent
    - module:mcp
  depends-on:
    - module:tool/bash
  consumes:
    - contract:tool-trait
    - contract:settings-json
source:
    - "src/permission.rs"
    - "src/preapproved.rs"
---
统一权限闸：模式（default / acceptEdits / auto / bypassPermissions / dontAsk / plan）
× 规则表（allow / deny / ask）× 工具属性 → allow / deny / ask。
刻意**不在** Tool trait 里（D2）——权限是横切关注点，工具只声明自己是什么。

规则语义里全是踩出来的细节：deny/ask 只要任一子命令命中就成立，allow 要求**全部**子命令命中；
Bash 按 shell 序列符（`&& || ; | &` 换行、子 shell 与命令替换定界符）切开后逐段前缀匹配，
引号内的分隔符不切、引号未闭合则整条不可信；文件类工具先做路径归一（`~` 展开、相对路径按 session cwd、
`.`/`..` 解析）再前缀匹配，且**不查文件系统**（规则对不存在的路径也必须成立）；
WebFetch 支持 `domain:` 与 URL 前缀；`mcp__server` 形式匹配整个 server。

`safety_check` 是免疫 bypass 的那一层：写类工具指向敏感目录一律弹窗。
`preapproved.rs` 是 WebFetch 的公共文档域白名单，只影响 GET 的权限判定，与网络沙箱无关（没有沙箱）。
