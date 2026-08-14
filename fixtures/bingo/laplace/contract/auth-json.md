---
tags: [contract, io]
source:
    - "src/auth.rs"
    - "src/api/auth.rs"
---
`~/.local/share/bingo/auth.json`，Unix 下 0600，**形状与 opencode 兼容**——
每个 provider 一条：`{"type":"oauth", access/refresh/expires/accountId}` 或 `{"type":"api", key}`。

刻意与 settings.json 分家：项目层设置是要提交的，凭据必须在另一个文件、另一个目录、另一套权限位。
`AuthSource::StoredKey` 每次请求现读，所以会话中途 `/provider login` 立刻生效，不用重启。

OAuth 侧（Codex/ChatGPT 的 device flow 与 loopback PKCE）做懒刷新 + 401 触发 + single-flight 锁，
永久失败就清掉登录态并提示重登；端点全部对着 openai/codex 源码核过（notes/research-oauth-cli.md）。
