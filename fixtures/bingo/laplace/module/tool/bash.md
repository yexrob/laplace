---
tags: [danger]
relations:
  part-of:
    - subsystem:tool-layer
  implements:
    - contract:tool-trait
  depends-on:
    - module:platform
    - module:watch
    - module:tasks
source:
    - "src/tool/bash.rs"
---
执行本地命令的工具，也是整个 harness 里危险面最大的一个。走 `tokio::process` + 平台 shell，
**没有 pty**；交互式/TTY 命令被主动拒绝（`interactive_command_reason` 维护 REPL 名单，
D32 之后包含 powershell/pwsh/cmd）——一个等待输入的进程在无 pty 下会永远挂着。

后台执行注册成 watchable，超时与取消走 `kill_process_tree`（见 module:platform 的杀序陷阱）。
输出有字符上限（`settings.bashOutputMaxChars`），超出截断。

D71 的教训值得记住：工具名 `Bash` 是模型看到的最强先验，在 Windows 上真正跑的是 PowerShell 时，
模型照样生成 POSIX 命令——修法不是改名（权限规则 `Bash(git push:*)`、hook、
存档 transcript、provider 侧的历史全都键在这个名字上），而是让工具描述和环境块**都报出解析后的真实 executor**。
