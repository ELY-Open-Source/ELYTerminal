# 开发

[English](../en_us/develop.md) / 中文

各平台的脚本把发行构建写到 `target/release`：在 macOS 上，`scripts/mac-app.sh` 构建带图标的 `ELYTerminal.app` 及其磁盘映像；在 Windows 上，`scripts/windows-zip.ps1` 把内含图标的 `elyterminal.exe` 连同 Microsoft 的 `conpty.dll`、`OpenConsole.exe` 及其许可打成 zip；在 Linux 上，`scripts/linux-packages.sh` 在 Ubuntu 22.04 的 Docker 里构建一次，再把同一份二进制打成 tar.gz、DEB、AppImage 和 RPM。`scripts/linux-tar.sh` 仍可单独构建压缩包。[Linux 打包](../../packaging/linux/README.md) 说明各脚本、校验和上传发行。

在 Linux 上，先安装 gpui 链接的库。Debian 和 Ubuntu：

```sh
sudo apt-get install pkg-config libwayland-dev libxkbcommon-dev libxkbcommon-x11-dev \
  libxcb1-dev libx11-xcb-dev libfontconfig-dev libfreetype-dev
```

应用把日志写到 stderr，自己的 info 行和 Ely 的。`RUST_LOG=elyterminal=debug,ely_gpui_component=info` 写得更多；`trace` 记录每一次绘制。Windows 的发行构建不打开控制台：`elyterminal.exe 2> log.txt` 留下日志。

`scripts/check.sh` 跑 fmt、clippy、测试和 500 行规则。其余在 [AGENTS.md](../../AGENTS.md)：布局、决定和工具。
