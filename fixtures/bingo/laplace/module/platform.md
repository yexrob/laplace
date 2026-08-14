---
tags: [cross-cutting]
source:
    - "src/platform.rs"
---
唯一的平台抽象层（D32）：shell 执行、进程树终止、TTY 查询，外加 D71 的 `ShellDialect`。

Unix 与 Windows 的进程语义不同，差异全部收在这里：Unix 把子 shell 起成自己的进程组组长
（pgid == shell pid），超时/取消杀整组，孙进程不会变孤儿；Windows 没有进程组，用 `taskkill /T` 杀整棵树，
**而且必须先杀树再杀根**——`taskkill /T` 需要根进程活着才能遍历，顺序反了就漏掉孙进程
（Unix 下顺序无所谓，所以这个 bug 在 mac 上永远复现不出来）。

默认 shell：macOS `/bin/zsh`、其他 Unix `/bin/bash`、Windows `powershell.exe`，`settings.shell` 可覆盖。
`ShellDialect`（posix / powershell / cmd / unknown）会同时进环境块和 Bash 工具描述——
不认识的 shell（比如 fish）诚实地报 unknown，而不是假定 POSIX。
无 unsafe 约束下 Unix 的杀组靠 `/bin/sh kill -pgid` 完成。
