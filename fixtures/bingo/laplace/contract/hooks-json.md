---
tags: [contract, io]
source:
    - "src/hooks.rs"
---
shell hook 的 JSON 契约：事件（PreToolUse / PostToolUse / UserPromptSubmit / SessionStart /
SessionEnd / Stop / PreCompact / PostCompact / TaskCreated / TaskCompleted）+ matcher +
stdin 收 JSON、stdout 返回决策。用户写的是任意可执行文件，所以这里的每一个字段名都是对外承诺。

关键能力：PreToolUse 可以**改写**工具输入或直接否决；TaskCompleted 的 blockingError 能把
「已完成」打回去。这让 hook 成为策略层而不只是通知层。

超时是纪律：普通 hook 有超时，SessionEnd 只给 1.5 秒快速拆卸——退出路径上等一个坏脚本，
用户感受到的是 bingo 卡死。
