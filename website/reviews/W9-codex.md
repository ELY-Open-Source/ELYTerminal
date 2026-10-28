FAIL

W9 审查，2026-10-28。已读取 PRD.md §§1–2、TASKS.md W9 及指定实现、样式和测试。发现两项可复现的要求缺口；语言/主题持久化、系统变化优先级、首次绘制及存储拒绝路径的独立检查通过。

**F1 — 中文导航的 Terminal 功能名称及可访问名称漏译**

文件：`website/index.html:28`；语言应用逻辑为 `website/src/preferences.js:49–55`。

根因：导航中指向 #terminal 的链接固定为 Terminal，缺少既有语言应用器所读取的 data-en/data-zh。切换语言后，该链接的可见文本和由文本产生的可访问名称均保持英文。

复现：使用无已保存选择的 zh-CN 浏览器上下文打开页面，或在英文页面点击“中文”。等待初始化及字体加载后，html 为 lang="zh-CN"、data-locale="zh"，导航显示 `Terminal / 会话 / Git / 下载`。实际可访问树为 navigation “主导航” 下的 link “Terminal”。1440×900 和 320×568、深浅两色均可复现；刷新后保持相同行为。

影响：功能导航及其可访问名称未完成要求的简体中文本地化。本项定位于导航功能名称。

最小修复：为该链接补充 data-en="Terminal"、data-zh="终端"，沿用现有 applyLocale()。在中文默认和手动切换路径中同时断言可见文本及可访问 link 名称，回切英文时恢复 Terminal。

证据：临时目录内 `quick-probe.json` 的中文 accessibility snapshot、`preferences-review.json` 的中文导航记录、`layout-zh-CN-light-1440.png`、`layout-zh-CN-dark-1440.png`。

**F2 — 320px 英文导航内部溢出，主题按钮键盘焦点环被视口裁切**

文件：`website/src/style.css:368–396`，涉及 `.site-header` 最大宽度、导航间距和控件间距；44px 控件位于 `:222–228`，焦点环位于 `:86–89`。测试遗漏位于 `website/tests/preferences.mjs:100–109`。

根因：320px 下 header 最大宽度为 304px，英文导航及两个固定 44px 控件所需空间超过其内部宽度。控件向右溢出；焦点环在按钮外再扩展 7px，超出视口。现有六尺寸测试在中文状态测量 header 和控件，切换英文后仅检查整个页面的 scrollWidth，因而遗漏内部溢出。

复现：在 320×568、en-US、无保存选择的上下文加载页面，等待 document.fonts.ready；连续按 Tab 到 #theme-toggle。也可先以中文加载，切换 EN 后再用 Tab 聚焦主题按钮。深浅两色均复现。

| 测量项 | 实际值 |
|---|---:|
| 页面 innerWidth / clientWidth / scrollWidth | 320 / 320 / 320px |
| header 左边 / 右边 | 8 / 312px |
| header clientWidth / scrollWidth | 302 / 306px |
| 主题按钮宽高 | 44 × 44px |
| 主题按钮右边 | 315.109375px |
| 焦点环 | 2px solid，outline-offset 5px |
| 焦点环最右边 | 322.109375px |

影响：主题控件越过导航容器右边，键盘焦点环右侧被 320px 视口裁切，违反窄屏无溢出/裁切要求。已目视确认深浅截图中的焦点环裁切。当前测试的 headerScroll ≤ headerClient + 1 及 button.right ≤ headerRight 两条断言在该英文状态均会失败。

最小修复：调整窄屏导航和控件的间距或可用宽度，使英文、中文均容纳两个完整 44px 控件，并留足焦点环空间。保留现有按钮尺寸。将完整 header/控件边界检查同时应用于两种语言的初始加载和切换后状态，并增加 Tab 聚焦主题按钮的窄屏检查。

证据：`quick-probe.json`、`preferences-review.json`、`focus-en-light-320.png`、`focus-en-dark-320.png`。中文同尺寸对照的 header clientWidth/scrollWidth 均为 256px，控件保持在容器内。

**独立验证结果**

行为探针覆盖 en-US、zh-CN、zh-TW、zh-Hans-CN 默认选择；所有中文输入均采用简体中文。通过连续 Tab 到达控件后，语言使用 Enter、主题使用 Space，再交换按键验证。两方向显式选择均在刷新后保留；手动主题选择后，后续系统深浅变化及刷新均以保存选择为准。控件名称随语言及目标主题更新。

分别模拟 localStorage 属性读取抛错、getItem 抛错、setItem 抛错，以及已存值无效。页面、Three 背景及两个控件继续工作；阻止读写时有明确警告，page errors 为空。无法保存时，本页手动主题选择仍优先于后续系统变化。另以一致的平台属性验证中文 Windows、Linux 及不支持平台的主下载标签和 href。

首次绘制使用本轮构建产物，经独立浏览器请求拦截在现有服务器源下读取；未启动服务器。保持主 JS 模块请求等待，允许 HTML/CSS 渲染，记录 first-paint 和 first-contentful-paint。系统深浅、相反保存选择、无效保存值及存储拒绝共八组全部通过：主模块尚未执行时，data-theme 与背景颜色已正确，浅色为 rgb(237,246,247)，深色为 rgb(8,23,33)。释放主模块后主题保持正确。

布局探针覆盖两语言 × 两主题 × 六视口，共 24 组：1440×900、1280×720、390×844、320×568、844×390、1280×600。两控件均为 44×44px，页面水平宽度符合视口；内部溢出和焦点裁切见 F2。已查看四种语言/主题组合的桌面截图、中文窄屏 Git 和下载截图，以及两色英文窄屏焦点截图。终端输出、路径和提交消息按示例内容核对。

**独立运行的既有检查**

安装、构建及全部测试在 `target/website-review-w9/run-qyczicgh/` 的源码副本执行，浏览器连接原有 http://127.0.0.1:5173。以下命令全部退出 0。

| Command | Result |
|---|---|
| `npm --prefix website ci --no-audit --no-fund` | 干净安装通过。 |
| `npm --prefix website test` | 7/7 通过。 |
| `npm --prefix website run build` | 15 个模块；JS 534.56 kB / gzip 134.57 kB；仅保留已接受的 Three chunk 警告。 |
| `node website/tests/preferences.mjs` | 现有语言、主题、持久化、键盘、存储及六尺寸断言通过。 |
| `node website/tests/visual.mjs` | 六视口通过，字体 loaded，errors / failures 为空。 |
| `node website/tests/ely.mjs` | 八张姿态捕获及减弱动画吸附通过。 |
| `node website/tests/content.mjs` | 内容、双语数据、鼠标/键盘页签及窗格末尾可达性通过。 |
| `node website/tests/scene.mjs` | 两色、八场景截图、减弱动画及 WebGL 回退通过；平均帧间隔 16.67ms。 |
| `node website/tests/downloads.mjs` | 平台、六链接、六布局及同页缩窗回归通过。 |

downloads.mjs 对平台标签断言显式设置 en-US，符合新增默认语言选择行为；资产目标断言保持原样。preferences.js、main.js、HTML 和 CSS 的改动源码没有新增注释或版本化 API，未发现新增死代码。重点手写文件均不超过 500 行，最长为 style.css 的 442 行。

全部独立脚本及证据位于上述临时目录：`website/quick-probe.mjs`、`website/preferences-review.mjs`、`website/prepaint-review.mjs`，对应 JSON 和日志文件；`commands.log`、`commands.json` 记录既有检查。回归截图在其 `target/website-shots/` 子目录。中文 Linux 下载探针首次输入保留了宿主 Mac 属性，修正测试输入为一致平台后重新运行通过，该测试输入问题未列入产品发现。

**输入标识与边界**

| Input | SHA-256 |
|---|---|
| `website/src/preferences.js` | `b0f2dd1527c639ca6fc6ba62060da29afd0ee02c4bd3ffd7aac47c3a2e1bd555` |
| `website/index.html` | `62f3e0ba6ce864d86e2b6e27b0e966630733d10778c563cb1ccddb3f3b9821a4` |
| `website/src/style.css` | `6c1762381792cd800a95d5e9c4d3eede5c5144b17cacfc3055d6fdf59675a04e` |
| `website/tests/preferences.mjs` | `d95ef52f31cd25bd0b31b2a30e5aef05cd13c3a4c16d74f22472b79373b7d7ff` |

除指定临时目录外，本轮仅新增本报告。起始文件及 44 张原截图的 SHA-256 已复核；实现、开发服务器、公开 release 和远端资源保持原样，独立浏览器已关闭。

浏览器证据来自 Chrome 156.0.8078.48。原生 Safari/Firefox、实体移动设备、实际读屏软件和生产环境未验证。
