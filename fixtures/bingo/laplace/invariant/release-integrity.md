---
tags: [discipline]
relations:
  binds:
    - subsystem:hosts
    - contract:error-codes
    - module:main
source:
    - ".github/workflows/**"
    - "scripts/check_release_version.py"
    - "scripts/smoke_release_archive.py"
    - "tests/cli_black_box.rs"
    - "Cargo.toml"
---
**发布 tag 必须精确等于 `v<package.version>`，且每个平台归档在发布前必须被解开、
在原生架构上真的跑一遍 `--version`。**（D35）

立这条之前，发布是从任意 `v*` tag 直接构建的：tag 与 Cargo.toml 可以不一致，
归档上传时没人执行过里面的二进制，真正的 CLI 进程边界完全在门禁之外。

现在的门：`scripts/check_release_version.py` 用标准库读 manifest，
畸形或不匹配的 tag 在任何构建之前就被拒；每个宿主跑
`cargo check/clippy -D warnings/test --locked --all-targets` 加独立的 fmt job；
`scripts/smoke_release_archive.py` 在原生 runner（含专门的 Intel macOS）上解包，
断言归档里**恰好一个** `bingo`/`bingo.exe`、`--version` 退出成功、
stdout 精确等于 `bingo <tag-version>`、stderr 为空。只有最后的发布 job 拿得到 `contents: write`。

`tests/cli_black_box.rs` 是同一条纪律的进程内延伸：跑真二进制，
断言 `--version`/`--help` 能绕过坏配置，以及一个典型的非 TTY 配置失败必须是
非零退出 + 空 stdout + 一行稳定的 `[error] code=CONFIG_INVALID msg=...` + 无 ANSI。
