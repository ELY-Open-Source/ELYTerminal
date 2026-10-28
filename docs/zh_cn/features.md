# 功能

[English](../en_us/features.md) / 中文

- 终端跑在 `alacritty_terminal` 上：256 色和真彩、粗体、暗淡、斜体、五种下划线、宽字符和组合标记。盒线、块元素和 powerline 形状按几何绘制，所以线段跨单元格对齐。按键遵循 xterm 和 kitty 键盘协议，在 Windows 上还有 win32-input-mode，和 Windows Terminal 发出的一样，所以 herdr 的快捷键能用；输入法在光标处组合；程序可以跟踪鼠标；程序要求时粘贴加括号；OSC 8 和纯文本链接用下面的链接点按打开；在 herdr 里由 herdr 打开，只开网页链接，其窗格折行处仍是整条，按住 Shift 则终端打开它所看见的；查找搜索回滚。
- 终端右缘的滚动条滚动历史，即使程序占用了滚轮。
- zsh、bash、fish 和 PowerShell 的 shell 集成，在 Windows 上也包括 Git for Windows 的 bash 和 `sh`，以及 Shell 为 `wsl` 时启动的 bash、zsh 或 fish，所以文件和 Git 能跟上 WSL 或 Git Bash 会话：每个提示符报告其文件夹和标题，OSC 133 标记每个提示符、每条命令及其退出状态。
- Ely 从窗口上缘探出头，窗口上方没有她的位置时则从左下角探上来，并且一直在动：呼吸、摇摆、眨眼。她跟着会话走：程序等你时她跳，出错时她抖，完成时她欢呼，程序工作时她顺着读，长久平静时她打盹。点按她，她眨眼，并把她从盹里叫醒。沿边缘拖动可挪她。设置 › 终端 › Ely 关掉她。
- 会话：新建、关闭和切换。右键点按一行可复制它，用它正在跑的 shell 和它上次报告的文件夹，或关闭它；拖动一行可换位置；设置 › 终端 可分别关掉这两项。响铃给隐藏会话的行做标记，不发声。一行的标记显示其程序通过 OSC 7501、OSC 9;4 或 herdr 报告的内容：等你、错误、带进度的工作、完成或空闲，否则是上一条失败的命令。标记是 3×3 的点：工作时涟漪，等待时下沉，持续到该状态结束。悬停在标记上列出每条报告。
- 通知：窗口在后台或会话隐藏时，会话开始等待、完成或失败、跑了 5 秒或更久的命令结束，以及程序的 OSC 9、777 或 99，会发一条系统通知，每个会话最多每 3 秒一条；点按显示该会话。在 macOS 上只有应用包会发通知，`cargo run` 不会。
- Cmd-Up 和 Cmd-Down，非 macOS 上是 Ctrl-Shift-Up 和 Ctrl-Shift-Down，跳到视口顶部之前或之后的提示符。
- macOS 上的自然文本编辑，设置 › 终端 里默认关闭：Option-Left 和 Option-Right 按词移动，Option-Backspace 和 Option-Delete 按词删除，Command-Left 和 Command-Right 移到行首行尾，Command-Backspace 和 Command-Delete 删到行首行尾，发出的是 shell 期望的编辑字节。
- 设置 › 终端 里的 Option 键作为 Alt 把所选的 macOS Option 键变成终端的 Alt，选了拼音之类的输入法时也一样；未选的 Option 键仍输入特殊字符。
- 标题栏两端各有一个按钮，隐藏或显示该侧栏；侧栏与终端之间的边框可拖动改宽度；窗口按离开时的大小和宽度打开。在 Windows 和 Linux 上，右侧按钮旁的齿轮打开设置。
- 文件把会话的文件夹显示为带 Git 标记的树。在 herdr 里，文件夹是 herdr 当前窗格的。
- Git 显示分支，暂存、取消暂存、丢弃、提交，列出最近提交并打开差异，全部通过 `git` 命令行。
- 主题：Ely Dark、Ely Light、Catppuccin Mocha、Macchiato、Frappé 和 Latte、Tokyo Night、Dracula、Nord、Gruvbox Dark、Rosé Pine、Solarized Light、One Light 和 Kanagawa Wave。系统为深色时可换第二套主题。在 Omarchy 4 上，Omarchy 主题跟着系统主题切换，并且是那里的默认。
- 背景不透明度和模糊：macOS 上是 NSVisualEffectView，Windows 上是 acrylic，Wayland 上是 `ext_background_effect_v1` 或 `org_kde_kwin_blur`，X11 上是 `_KDE_NET_WM_BLUR_BEHIND_REGION`，KWin 会读它。模糊只在不透明度低于 100% 时出现。Hyprland 只在自己的 `decoration:blur:enabled` 打开时模糊，而 Omarchy 4.0.4 把它关掉了；要在那里打开，把 `hl.config({ decoration = { blur = { enabled = true } } })` 加到 `~/.config/hypr/looknfeel.lua`。
