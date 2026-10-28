# Linux

[English](../en_us/install.md) / 中文

把 `VERSION` 换成该发行版的版本号。Linux 构建需要 glibc 2.35 或
更高、X11 或 Wayland 桌面，以及可用的图形驱动。系统软件包通过包管理器
安装运行库、桌面项和图标。下列格式可用
`scripts/linux-packages.sh` 构建。请用所选发行版实际列出的资源；
打包 CI 的产物是另外的下载。

在 Ubuntu 22.04、Ubuntu 24.04 或 Debian 12 上：

```sh
sudo apt install ./elyterminal_VERSION_amd64.deb
```

在 Fedora 43 上：

```sh
sudo dnf install ./elyterminal-VERSION-1.x86_64.rpm
```

更新时用同样方式安装更新的软件包。卸载用
`sudo apt remove elyterminal` 或 `sudo dnf remove elyterminal`。
软件包操作不会动 `~/.config/ELYTerminal`。

AppImage 无需系统安装即可运行：

```sh
chmod +x ELYTerminal-VERSION-linux-x86_64.AppImage
./ELYTerminal-VERSION-linux-x86_64.AppImage
```

它自带键盘库，但使用主机的 glibc、X11/Wayland、
图形库和已安装的字体。主机上需安装 `git`、`curl` 和 `xdg-utils`，
才能用 Git、检查更新和打开桌面链接。没有 FUSE 时，运行
`./ELYTerminal-VERSION-linux-x86_64.AppImage --appimage-extract-and-run`。
更新则替换该文件；卸载则删除它。其他发行版和
Wayland 桌面需在各自的图形栈上试过。

压缩包仍可用。先安装 libxkbcommon、libxkbcommon-x11、libxcb、
桌面的运行库和上面那些工具：

```sh
tar -xzf ELYTerminal-VERSION-linux-x86_64.tar.gz
cd ELYTerminal-VERSION-linux-x86_64
install -Dm755 elyterminal -t ~/.local/bin
install -Dm644 com.zacharyzhang.elyterminal.desktop -t ~/.local/share/applications
install -Dm644 com.zacharyzhang.elyterminal.svg -t ~/.local/share/icons/hicolor/scalable/apps
install -Dm644 elyterminal.1.gz -t ~/.local/share/man/man1
```

桌面项运行 `elyterminal`，所以 `~/.local/bin` 必须在 `PATH` 上。
用更新的压缩包再跑一遍这些命令即可更新。卸载则删掉
这四个已安装的文件；设置仍留在配置文件夹里。
