FAIL

W7 第 1/3 轮审查，2026-10-28。本轮限定为代码、测试、布局及现有发布脚本，发现两个可复现的验收阻塞。

**F1 — iPad 桌面浏览模式被识别为 macOS，主下载错误指向 DMG**

文件：`website/src/downloads.js:21–26`，影响 `:42–50` 的主链接与文案。

根因：unsupported 判断仅检查 user-agent/platform 中的 Android、iPad、Mobile 等字符串。iPad Safari 的桌面浏览模式使用 macOS user-agent，platform 为 MacIntel，代码随后进入 macOS 分支，且忽略可用于区分该输入的 maxTouchPoints。WebKit 官方说明了 iPad Safari 通常发送与 macOS 相同的 user-agent：[Desktop-class Browsing on iPad](https://webkit.org/blog/9674/new-webkit-features-in-safari-13/)。

复现输入：

```js
detectPlatform({
  platform: 'MacIntel',
  userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) '
    + 'AppleWebKit/605.1.15 (KHTML, like Gecko) '
    + 'Version/18.6 Safari/605.1.15',
  maxTouchPoints: 5,
  userAgentData: undefined,
});
```

实际返回 `macos`，预期为 `other`。独立浏览器在页面脚本执行前注入同一组 navigator 属性，实际得到 `data-platform="macos"`、`Download for macOS` 和 `https://github.com/ELY-Open-Source/ELYTerminal/releases/latest/download/ELYTerminal-macos-arm64.dmg`。相同 UA、maxTouchPoints=0 的 macOS 对照仍返回 macos；带 iPad/Mobile 的移动 UA 返回 other。

影响：不支持的 iPadOS 主按钮提供无法运行的 macOS 安装包，违反“不支持的平台去 releases 页面”的要求。此结果来自实际函数和页面逻辑的可重复属性仿真，实体 iPad 未测试。

最小修复：在 macOS 分支前处理桌面模式 iPad，例如联合判断 MacIntel 与 maxTouchPoints>1，将其归入 other；保留普通 macOS 的 DMG 映射。增加对应单元输入及浏览器主链接断言，验证结果为 releases 页面和 View releases。现有测试只覆盖包含 iPad/Mobile 字符串的输入。

证据：`target/website-review-w7/run-2x_ib28_/independent-downloads.log`、`independent-downloads.json`、`ipad-desktop-download.png`。

**F2 — 短屏滚至下载区底部后，标题和操作区域与固定 Ely 重叠**

文件：`website/src/scenes.css:315–324`；相关布局为 `website/src/style.css:242–254`、`website/src/ely.js:53–66`。

根因：短屏分支将 Download 场景扩为 48rem，标题及操作区域分别定位在文档场景的 28rem、39rem。为显示下载链接，页面继续向下滚动，这两块内容随文档上移；Ely 保持固定定位，并保持 Download 姿态的视口位置和尺寸。短屏新增布局未协调这两套坐标。

复现：分别使用 844×390、1280×600，滚至 Download 场景，再在页面右侧滚轮向下 1000px 至页面底部，等待姿态稳定。实测与截图如下：

| 视口 | 页面 scrollY | 可见重叠 |
|---|---:|---|
| 844×390 | 2490px | 标题位于 y=71–135.98，与 Ely 头部相交约 156.42×64.98px；主按钮 y=247–303，与 Ely 相交约 191.83×52.01px。 |
| 1280×600 | 2280px | 标题位于 y=281–379.55，与 Ely 所持终端及身体区域相交约 262.78×98.55px。 |

已目视检查截图：844×390 的 Download 横穿头部，主按钮盖住人物下部；1280×600 的标题横穿 Ely 所持终端。这违反本轮下载布局不得重叠的要求。链接点击命中正常，故此项归因于布局重叠。

最小修复：协调短屏 Download 姿态的位置/尺寸与标题、操作区布局，保证原生滚动到可访问全部下载链接的位置时，Ely、标题和主按钮仍保持分离。回归应覆盖上述两个视口实际滚到底后的重叠检查与截图。当前 `downloads.mjs` 只比较标题、actions 和 section 边界，独立布局探针补查 Ely 后复现该缺口。

证据：`target/website-review-w7/run-2x_ib28_/overlap-probe.json`、`overlap-844-390.png`、`overlap-1280-600.png`。复验使用真实滚轮，记录实际页面末尾状态。

**独立执行结果**

临时源码、依赖、构建、日志、浏览器临时目录及截图全部位于 `target/website-review-w7/run-2x_ib28_/`。指定开发服务器保持运行。安装与构建在该目录的源码副本执行；发布脚本测试将 cargo、gh 替换为本地桩，未调用真实上传命令。

| Command | Result |
|---|---|
| `npm --prefix website test` | 退出 0，6/6 通过。 |
| `npm --prefix website run build` | 退出 0，14 个模块；JS 534.27 kB / gzip 134.41 kB，保留已接受的 Three chunk 警告。 |
| `node website/tests/visual.mjs` | 退出 0；六视口通过，字体 loaded，errors / failures 为空。 |
| `node website/tests/ely.mjs` | 退出 0；八张姿态捕获及减弱动画吸附通过。 |
| `node website/tests/content.mjs` | 退出 0；六视口、双语数据、页签及 W6 末尾可达性回归通过。 |
| `node website/tests/scene.mjs` | 退出 0；深浅场景、减弱动画和 WebGL 回退通过；帧间隔均值 16.67ms。 |
| `node website/tests/downloads.mjs` | 退出 0；既有四种平台选择、六个链接和六视口断言通过；遗漏路径见 F1/F2。 |

干净 `npm ci` 也通过。独立平台探针覆盖 macOS Safari、Windows/Linux 的传统属性路径、iPad 桌面/移动模式、iPhone、Android、ChromeOS、FreeBSD，发现 F1，其余结果符合要求。

独立下载布局检查覆盖 1440×900、1280×720、390×844、320×568、844×390、1280×600，在页面底部测量标题及七个链接。页面宽度三项一致，链接均在视口内、可命中、彼此分离；逐个点击在本地阻止导航并记录 href，六个格式目标完全匹配规定 URL。Ely 与这些区域的相交另由 F2 记录。已查看提供的 `download-1440.png`、`download-390.png`，以及独立生成的窄屏、横屏和 F1/F2 截图。

**发布脚本及既有约束**

六个稳定别名和 `releases/latest/download/ASSET` 路径均准确。普通 macOS 主链接选择 DMG，Windows 选择 x86_64 ZIP，Linux 选择 AppImage；运行时下载代码及链接没有硬编码 release 版本。

发布脚本独立测试使用另一版本 2.4.6 和含空格的资产目录，覆盖六种格式的普通上传及 alias-only，共 12 个映射场景。普通模式依次上传版本化资产和正确别名，alias-only 只上传别名；桩在上传时读取文件并核对 SHA-256，别名字节均与原文件相同。未知 alias-only 输入、缺失文件、缺失参数和多余参数均在上传前失败。分别注入首次上传、第二次上传和 alias-only 上传失败，脚本均非零退出，临时别名目录得到清理。

`scripts/release-asset.sh` 继续由现有 Release workflow 调用，仓库未引入第二套发布系统；`packaging/linux/README.md:176–180` 记录了版本化上传、稳定别名及 alias-only 行为。相应证据在 `release-probe.py`、`release-probe.json`。

与 W6 第 2 轮源码比较，Ely rig、帧调度、背景、页签模块及终端 CSS 保持相同；既有回归全部通过，新增布局影响见 F2。当前界面保持功能窗格及简洁下载链接，没有新增营销嵌套卡片或无用描述。重点手写文件均低于 500 行，最长为 scenes.css 的 326 行；所检查运行时文件最长为 terminal.css 的 385 行。发布脚本新增注释为一行八个英文词；其余本轮源码没有新增注释。

**输入标识与审查边界**

| File | SHA-256 |
|---|---|
| `website/src/downloads.js` | `4d641885ba5244396b182bc4a32c02f720f6671ec702a22238470e2fd74264e6` |
| `website/src/scenes.css` | `bda8039275ff968ae488376d96a188a756c1edf4700d33a5821f2d0649a29178` |
| `website/index.html` | `056a391be21e74246282fae58f543c98eac28b6dd1fc121921237539b9b0d60e` |
| `scripts/release-asset.sh` | `9d98069dec5a0c2eb3598b29478c5311095c44ebd39ee7bc0d5ee48fd4b83c08` |
| `packaging/linux/README.md` | `c85517939c261e897d734be5ef92c1563f55fe66c290e248cb554eb646730d03` |

除指定临时目录外，本轮仅新增本报告；起始文件及 37 张原截图的 SHA-256 已复核。本轮未修改实现、公开 release、远端资源及开发服务器。独立浏览器已关闭。

公开别名上传与 redirected HEAD 验证待所有者授权后进行，本轮未验。该待办按本轮明确范围处理。实体 iPad、原生 Safari/Firefox 及公开下载内容未测试；F1 的平台输入仿真和 F2 的 Chrome 布局证据已分别注明。
