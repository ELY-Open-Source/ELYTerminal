PASS

W9 第 2 轮复审，2026-10-28。上一轮 F1、F2 均已关闭，本轮独立检查未发现阻碍 W9 验收的回归。

已重读上一轮报告、PRD.md §§1–2、TASKS.md W9、index.html、preferences.js、main.js、downloads.js、三个样式文件及相关测试。与上一轮审查快照相比，实现和测试仅有 index.html、src/style.css、tests/preferences.mjs 三个文件变化。

**F1 已关闭。** `website/index.html:28` 使用既有 data-en/data-zh 属性提供 Terminal/终端。独立浏览器以 zh-CN 默认加载，在 1440×900 和 320×568、深浅两色下，链接可见文本与可访问名称均为“终端”。en-US 默认加载及回切英文时均为 Terminal。鼠标切换、键盘切换和刷新后的保存选择均保持正确。实际可访问树中，中文导航为 navigation “主导航” 下的 link “终端”，目标仍为 #terminal。

**F2 已关闭。** `website/src/style.css:424–428` 在宽度 ≤22rem 时减少导航链接水平内边距。按原路径等待字体加载，在 320×568 英文页面连续按 Tab 到主题按钮；深浅两色的焦点环均完整可见。另从中文页面点击 EN，再按 Tab 到主题按钮，得到相同英文边界。

| 320×568 测量项 | 英文 | 简体中文 |
|---|---:|---:|
| innerWidth / clientWidth / scrollWidth | 320 / 320 / 320px | 320 / 320 / 320px |
| header 左边 / 右边 | 13.109375 / 306.890625px | 51.234375 / 268.765625px |
| header clientWidth / scrollWidth | 292 / 292px | 216 / 216px |
| 两个控件宽高 | 各 44×44px | 各 44×44px |
| 主题按钮右边 | 301.09375px | 262.96875px |
| 键盘焦点环右边 | 308.09375px | 269.96875px |

焦点样式仍为 2px outline、5px offset，实际 :focus-visible 为 true。上一轮英文 header 内部溢出及焦点环右边 322.109375px 的问题已消除。已目视检查本轮英文两色窄屏焦点截图、中文浅色桌面和中文深色窄屏截图，导航、文字和控件保持可读。

独立布局探针覆盖两语言 × 两主题 × 六视口，共 24 组：1440×900、1280×720、390×844、320×568、844×390、1280×600。全部检查通过：页面宽度与视口一致，header 内部内容和控件保持在边界内，两控件均为 44×44px；滚至 Sessions、Git 和 Download 后保持页面宽度。更新后的 preferences.mjs 在六视口等待字体后，对中文初始状态和英文切换状态分别执行完整 header/控件边界检查，并检查主题按钮焦点环。

语言、主题及异常回归均通过。独立上下文覆盖 en-US、zh-CN、zh-TW、zh-Hans-CN；中文浏览器均默认简体中文。连续 Tab 可到达两个控件，语言 Enter、主题 Space 生效，交换为语言 Space、主题 Enter 也生效。两方向语言选择及深浅主题选择均在刷新后保留，控件可访问名称随语言和目标主题更新。尚未手动选择时主题跟随系统；手动选择后，后续系统深浅变化及刷新均保留保存选择。

分别模拟 localStorage 属性访问抛错、getItem 抛错、setItem 抛错和无效保存值。页面、背景与两个控件正常工作，page errors 为空；受阻读写有明确警告。保存失败时，本页手动主题仍优先于之后的系统变化。中文 Windows、Linux 和不支持平台的主下载标签及链接检查通过。

首次绘制使用本轮构建产物，通过浏览器请求拦截在原服务器源下读取，保持主 JS 请求等待。系统深浅、相反保存选择、无效保存值、存储拒绝共八组通过：first-paint/first-contentful-paint 时主题已正确，浅色背景 rgb(237,246,247)，深色 rgb(8,23,33)；此时主模块尚未设置 locale。放行主模块后主题保持正确。

全部安装、构建和测试在 `target/website-review-w9-r2/run-k7wx4uuy/` 的源码副本中独立执行，浏览器连接现有 `http://127.0.0.1:5173`。

| Command | Result |
|---|---|
| `npm --prefix website ci --no-audit --no-fund` | 干净安装通过。 |
| `npm --prefix website test` | 7/7 通过。 |
| `npm --prefix website run build` | 15 个模块；JS 534.56 kB / gzip 134.57 kB；保留已接受的 Three chunk 警告。 |
| `node website/tests/preferences.mjs` | 双语、主题、持久化、键盘、存储及六尺寸通过。 |
| `node website/tests/visual.mjs` | 六尺寸通过；字体 loaded；errors / failures 为空。 |
| `node website/tests/ely.mjs` | 八张场景捕获及减弱动画吸附通过。 |
| `node website/tests/content.mjs` | 六尺寸、双语内容、鼠标/键盘页签和窗格末尾可达性通过。 |
| `node website/tests/scene.mjs` | 深浅色、八场景、减弱动画及 WebGL 回退通过；平均帧间隔 16.67ms。 |
| `node website/tests/downloads.mjs` | 五种平台选择、六链接、六布局及同页缩窗回归通过。 |

本次两处实现修复沿用现有语言应用器及响应式样式，没有新增 API、注释或死代码。重点手写文件均在 500 行以内，最长为 style.css 的 448 行。

证据位于上述临时目录：`commands.log`、`commands.json`；独立脚本 `website/quick-probe.mjs`、`website/preferences-review.mjs`、`website/prepaint-review.mjs` 及对应 JSON/日志。`quick-probe.json` 记录默认和切换后的导航、可访问树与真实 Tab 焦点；`preferences-review.json` 记录 24 组布局、四种浏览器语言、存储异常和下载标签；`prepaint-review.json` 记录八组首次绘制。窄屏焦点截图为 `focus-en-US-light-320.png`、`focus-en-US-dark-320.png`、`focus-zh-CN-dark-320.png` 和 `quick-switched-focus-zh-CN.png`；回归截图在临时目录的 `target/website-shots/` 下。

| Input | SHA-256 |
|---|---|
| `website/index.html` | `3a626fdad3ce7eb7eef6779e1e6ad0cf8e53c4ed1fd6db03fe6f49fc7c213050` |
| `website/src/style.css` | `6404ad7b900dd77caa8a9f5b57c9e909daac99ed7a2ac682d319de446307919f` |
| `website/tests/preferences.mjs` | `a6caa42d893261f49722971b12ca3f9e05a8ea253de60154c377b121f7005d5a` |
| `website/src/preferences.js` | `b0f2dd1527c639ca6fc6ba62060da29afd0ee02c4bd3ffd7aac47c3a2e1bd555` |

原有 4075 个文件及 44 张原截图已按 SHA-256 核对。指定临时目录外仅新增本报告；实现、开发服务器、公开 release 和远端资源保持原样。独立浏览器均已关闭。

浏览器证据来自 Chrome 156.0.8078.48。原生 Safari/Firefox、实体移动设备、实际读屏软件及生产环境未验证。
