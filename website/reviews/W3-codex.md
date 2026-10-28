FAIL

W3 第 1/3 轮审查完成。确认 2 个响应式布局阻塞。

范围：`website/index.html`、`src/style.css`、`src/terminal.css`、`src/main.js`、`tests/visual.mjs`，以及指定的三张 hero 截图。依据为 W3 的响应式 hero、terminal scene、navigation 和内容约束。审查日期：2026-10-28。

**F1 · P2 · 短视口下终端窗口遮住标题和 Ely，纵向滚动无法恢复内容**

位置：`website/src/style.css:150–168`、`website/src/terminal.css:1–12`、`website/src/terminal.css:377–384`。

复现：将页面设为 844×390，即指定 390×844 手机视口的横屏方向，等待字体加载完成。终端窗口的边界为 `x=109.72, y=38.41, width=696.31, height=336`；标题第二行的边界为 `x=37.97, y=129.09, width=479.36, height=82.30`。窗口覆盖该行约 407.61×82.30 px，截图中产品名大部分被盖住，Ely 也位于窗口后方。此时 document scrollHeight 与 innerHeight 均为 390，执行 `window.scrollTo(0, 9999)` 后 scrollY 仍为 0。

相同根因也出现在 1280×600：窗口从 y=240 开始，标题到 y=289.09，标题第二行约 421.02×49.09 px 被覆盖。

原因：hero 只以 100dvh 撑高，标题、Ely 和终端全部绝对定位；终端以底边定位并保留最小高度，短高度媒体规则仍要求至少 21rem。内容挤占同一位置，hero 的文档流高度保持一屏。

修正：让短视口布局为标题、Ely 和终端保留可见空间；需要额外高度时，通过页面原生纵向滚动访问。复验 844×390、1280×600，并保持指定的三个视口通过。此问题属于 W3 当前布局。

**F2 · P2 · 320px 手机宽度截断产品名，现有溢出断言漏报**

位置：`website/src/style.css:50–54`、`website/src/style.css:222–233`、`website/tests/visual.mjs:29–45`。

复现：Chrome 手机仿真 320×568，等待字体加载。标题从 x=20 延伸到 x=341.06，实际可视宽度为 320；查看 320×568 的视口截图可见 `TERMINAL` 右侧被裁切。执行 `window.scrollTo(500, 0)` 后 scrollX 仍为 0，无法查看被裁切的部分。

原因：移动标题的 `clamp(3.25rem, 14.5vw, 6rem)` 在该宽度采用 52px 下限，标题超过剩余宽度；移动 Ely 容器的 `right: -5vw` 同时伸出视口。测量如下：

| Requested viewport | clientWidth | scrollWidth | innerWidth | Title right | Ely right |
|---|---:|---:|---:|---:|---:|
| 390×844 | 390 | 410 | 410 | 369.14 | 409.50 |
| 320×568 | 320 | 341 | 341 | 341.06 | 336.00 |

`visual.mjs:36` 比较 `scrollWidth > innerWidth`，这里两者一同扩大，检查得到 false。指定的 `target/website-shots/hero-mobile.png` 实际 PNG 尺寸为 410×888；fullPage 捕获将溢出区域纳入截图，掩盖了它与指定 390×844 视口的差异。

本轮仅在浏览器内临时覆盖样式定位原因：390px 下将 Ely 的 right 设为 0，clientWidth / scrollWidth / innerWidth 恢复为 390 / 390 / 390；320px 下还需让标题采用 14.5vw，三者才恢复为 320 / 320 / 320。仓库 CSS 保持原状。

修正：让 320px 下产品名完整落入视口，并约束移动 hero 元素的横向边界；截图检查应以配置的视口宽度或 documentElement.clientWidth 检查溢出。复验实际视口截图中完整产品名可见、宽度保持正确，覆盖 320×568 和 390×844。

**其余范围验证**

| Check | Result |
|---|---|
| 指定视觉证据 | 已查看 hero-desktop.png、hero-laptop.png、hero-mobile.png。桌面与笔记本图分别为 1440×900、1280×720，符合其视口；手机图的尺寸差异见 F2。 |
| 原截图脚本 | 在仓库外临时副本中，对新生产构建运行原 `tests/visual.mjs`，退出 0；三组均报告 fonts loaded、overflow false、errors 空。该结果与 F2 的漏报原因一致。 |
| 实际导航 | 六个视口均通过 skip link 的 Tab 聚焦及 Enter 到 #content、Terminal 到 #terminal。GitHub 点击导航到精确仓库 URL；测试拦截该外部请求，验证 href 与浏览器导航行为。 |
| 构建与运行 | 隔离副本的 npm ci 和 production build 均退出 0；JS 为 523.42 kB / gzip 130.28 kB，尺寸提示保留为已知警告。Chrome 156.0.8078.48 中六个视口的三种字体均 loaded，page errors 均为空。 |
| 内容与职责 | 页面以产品名、导航和终端应用预览组织内容；预览内部的会话、Git 控件和命令输出属于产品画面。未发现嵌套营销卡片、tagline 或 filler copy。 |

补充视口为 320×568、844×390、1280×600。原始截图、完整页面截图和 320×568 实际视口截图均已目视核对。最终姿态、synthwave 背景、其他段落、下载与 locale/theme controls 继续归 W4–W8。

**输入标识**

| Input | SHA-256 |
|---|---|
| `website/index.html` | `575ef2048e935cc8fc61db5e7ad065c813aefe1361348941eeebe6b9b96430b2` |
| `website/src/style.css` | `326954e1ff734d7af86b11c61e8e19a69157a75fe008dcf78791410e1648de55` |
| `website/src/terminal.css` | `d6e4fc46cfc5af5206eda6f7fb807249b09feea2bdf69c5936cb93a6a769d351` |
| `website/src/main.js` | `dd6f5bee9c7a09c0ccef4db1ef727aae5f60800db4baca81b99d852b03d0a73b` |
| `website/tests/visual.mjs` | `23e68126462f06ad34f0be0b654096232d5e29c7c02f31eccf9e2d498f67ca96` |

本轮使用临时副本构建、运行和捕获，浏览器及服务器已关闭，临时副本随后清理；原始截图保持原状，仓库写入范围为本报告。

未测试 Safari、Firefox、实体手机或生产部署；手机结论来自 Chrome 手机仿真及对应像素截图。当前风险为 F1/F2 所述内容遮挡、裁切及视觉检查漏报。

