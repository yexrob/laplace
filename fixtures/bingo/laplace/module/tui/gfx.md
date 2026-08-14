---
tags: [ui]
relations:
  part-of:
    - subsystem:image-pipeline
source:
    - "src/tui/gfx.rs"
---
kitty 图形协议：能力探测 + 序列构造 + 图片加载 + placement 层的 diff 收敛
（desired vs active：每 instance 传一次 `a=T`，移动只重发 place，消失发 `a=d,d=I`）。

只有一种放置方案：Unicode 占位符（`U=1`）。图片数据按 id 传一次，
出现的格子就是普通带样式的文本（占位字符 + 行列变音符号 + 藏在前景色里的 image id），
由渲染层像画普通文本一样画出来——所以重绘、滚动、裁剪、multiplexer 重画全都自动正确，
零 placement 簿记（D42 删掉了 C=1 直放路径）。变化的只有传输方式（Bare / tmux passthrough）。

协议约束是review 抠出来的（D37）：kitty 的 `x=`/`y=` 是**源图裁剪键不是屏幕坐标**，
图片画在光标格上，所以定位必须留在 term.rs；每个 put 都要写 `p=1`，
否则同 id 再 put 会多出一份拷贝而不是移动；每条命令都要 `q=2`，
因为 `q=1` 的 APC 报错回包会以键盘输入的身份出现在 stdin 上，把输入框刷爆并触发重绘死循环。

代价：WezTerm/Konsole 因为只支持 C=1 路径而失去了图片支持。
