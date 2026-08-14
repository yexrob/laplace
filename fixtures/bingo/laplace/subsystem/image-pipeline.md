---
tags: [ui]
relations:
  part-of:
    - subsystem:rendering
  depends-on:
    - module:tui/gfx
    - module:tui/term
source:
    - "src/tui/gfx.rs"
    - "src/api/image.rs"
    - "src/tui/avatar.rs"
---
从磁盘/网络上的图片到终端上真的显示出来，再到发给模型之前的预处理，是一条链上的两头。
显示端只有一种放置方案：kitty 的 Unicode 占位符（`U=1`）——图片数据按 id 传一次，
出现的格子就是普通带样式的文本（占位字符 + 行列变音符号 + 藏在前景色里的 id），
所以重绘、滚动、裁剪、multiplexer 重画全都自动正确，零放置簿记（D42 删掉了 C=1 直放路径）。

发送端 `api::image` 把附件缩到 API 上限再编码（2000×2000 / ~3.75MB 原始字节，
base64 后不超 5MB 硬限）。头像是同一条路的小尺寸用法，不是第二套机制。

包袱：resize 会让终端遗忘所有 placement 和图片数据，两个宿主都必须走 force-redraw 重传（D37）；
tmux 占位模式在实时视口里退回 `#[image]` 行；每条命令都要带 `q=2`，
因为 `q=1` 的 APC 报错回包会以键盘输入的身份出现在 stdin 里，把输入框刷爆并触发重绘死循环。
