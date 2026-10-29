# ELYTerminal

[![Release](https://img.shields.io/github/v/release/ELY-Open-Source/ELYTerminal?style=flat-square&color=24292f)](https://github.com/ELY-Open-Source/ELYTerminal/releases/latest)
[![License](https://img.shields.io/badge/license-MIT-3da639?style=flat-square)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.95-dea584?style=flat-square)](https://www.rust-lang.org)
[![GPUI](https://img.shields.io/badge/GPUI-0.3-5b4cff?style=flat-square)](https://crates.io/crates/gpui-pre)
[![Ely](https://img.shields.io/badge/Ely-components-6d28d9?style=flat-square)](https://github.com/ELY-Open-Source/Ely-GPUI-Components)
[![alacritty](https://img.shields.io/badge/alacritty-0.26-f46d01?style=flat-square)](https://github.com/alacritty/alacritty)

[English](README.md) / 中文

面向 macOS、Windows 和 Linux 的终端，用 Rust 写在 GPUI 和 Ely 组件上。左边是会话，中间是终端，右边是 Git 或文件。每个设置都有界面控件。

## 安装

[GitHub 上的发行版](https://github.com/ELY-Open-Source/ELYTerminal/releases) 为每个平台提供构建。

- macOS 11 或更高，Apple silicon：打开 `ELYTerminal-VERSION.dmg`，把 ELYTerminal 拖进应用程序。应用已用 Developer ID 签名并公证。
- Windows 10 或更高，x86_64：解压 `ELYTerminal-VERSION-windows-x86_64.zip`，运行 `elyterminal.exe`，zip 里的其他文件留在它旁边。它未签名；若 SmartScreen 拦住，点按“更多信息”，再点按“仍要运行”。
- Linux，x86_64：[软件包](docs/zh_cn/install.md)。

## 功能

- 左边是会话，中间是终端，右边是 Git 或文件。
- zsh、bash、fish 和 PowerShell 的 shell 集成，文件和 Git 跟着会话的文件夹。
- Ely 从窗口边缘探出头，跟着会话走。
- 通过 kitty 图形协议显示图片，包括动画；通过 kitty 的 OSC 5522 读写剪贴板，程序读取其中内容前先问你。
- macOS、Windows、Wayland 和 X11 上的背景不透明度和模糊，Windows 11 上还有云母。

细则见 [docs/zh_cn/features.md](docs/zh_cn/features.md)。

## 快捷键

设置 › 终端 列出全部快捷键，按本平台的绑定。表在 [docs/zh_cn/keys.md](docs/zh_cn/keys.md)。

## 设置

设置窗口把每一次改动立刻存到平台配置文件夹里的 `settings.json`：

- macOS：`~/Library/Application Support/ELYTerminal/settings.json`
- Linux：`$XDG_CONFIG_HOME/ELYTerminal/settings.json`，否则 `~/.config/ELYTerminal/settings.json`
- Windows：`%APPDATA%\ELYTerminal\settings.json`

文件解析失败，或某个值超出范围，启动会停下来，错误指出文件和字段。应用从不覆盖它。

设置 › 外观 › 语言 把界面显示为 English 或简体中文。

设置 › 终端 在启动时和每天检查一次是否有更新，或用立即检查马上查；有新版本时出现下载按钮，打开其页面。检查通过 `curl` 进行，macOS 和 Windows 自带。

## 构建

Rust 1.95.0，由 `rust-toolchain.toml` 钉死；第一次构建时 rustup 会装上。

```sh
cargo run --release
```

发行脚本、Linux 库和日志在 [docs/zh_cn/develop.md](docs/zh_cn/develop.md)。

## 许可

MIT，见 `LICENSE`。盒线和按键编码沿用 Ghostty 的代码，MIT，见 `LICENSE-GHOSTTY`。主题文件来自 iTerm2-Color-Schemes，MIT，其许可在 `assets/themes`。JetBrains Mono 及其 Nerd Font 构建遵循 SIL Open Font License，见 `assets/fonts`。复合文本解码器的表来自 libX11，遵循 SunSoft 和 Bruno Haible 的声明，见 `vendor/xim-ctext/tables/LICENSE`。Microsoft 的 ConPTY，即 `conpty.dll` 和 `OpenConsole.exe`，为 MIT，见 `vendor/conpty/LICENSE`。
