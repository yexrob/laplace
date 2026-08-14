---
tags: [contract, io]
source:
    - "src/settings.rs"
---
三层配置：用户层 `~/.config/bingo/settings.json`、项目层 `.bingo/settings.json`（要提交）、
本地层 `.bingo/local.json`（永不提交），浅合并（D9）。承载 permissionMode、hooks、mcpServers、
providers、theme、experimental 开关等。

分层本身带语义：settings 管「要不要启动」（如 `team.autoStart`），team.json 管「启动什么」。
凭据绝不进这里——项目层是会被提交的，apiKey 泄进版本库这条路从格式上就被堵死了。

包袱：合并按字段进行，实验开关是「任一层为 on 即 on」；未知顶层键会静默生效为空操作，
所以启动时有一道 config lint 把拼错的键和悄悄回退的枚举值报成 startup note。
