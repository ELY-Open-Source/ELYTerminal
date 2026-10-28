PASS

W3 第 2/3 轮复审完成。F1、F2 均已关闭；本轮剩余 W3 阻塞为 0。审查日期：2026-10-28。

范围：响应式 hero、terminal scene、navigation，以及嵌套营销卡片和 filler copy 约束。已重读 `website/index.html`、`src/style.css`、`src/terminal.css`、`src/main.js`、`tests/visual.mjs`，对照 W3 任务和首轮报告，并查看 `target/website-shots` 中六张初始视口截图及三张短屏滚动截图。

**F1：关闭**

`src/style.css:236–240` 为短视口提供 44rem 文档流高度；`src/terminal.css:9` 将桌面终端高度设为 53vh，`src/terminal.css:377–385` 将短桌面标题起点设为 6rem。移动端保留其 48vh / 20rem 覆盖规则。

844×390 下，标题底边为 260.59px，终端顶边为 352.41px；1280×600 下分别为 313.09px 和 344px。两组标题均位于终端上方，标题起点 96px 也低于导航链接底边 69px。三种短屏的文档高度均为 704px。

本轮独立探针通过浏览器滚轮实际滚到页面底部，等待滚动到达终点，再断言终端窗口的上下边界都位于视口内：

| Viewport | scrollHeight | Final scrollY | Terminal top | Terminal bottom |
|---|---:|---:|---:|---:|
| 320×568 | 704 | 136 | 230.97 | 550.97 |
| 844×390 | 704 | 314 | 38.41 | 374.41 |
| 1280×600 | 704 | 104 | 240.00 | 576.00 |

首轮的标题被终端覆盖且页面无法纵向滚动的根因已解除，截图与实际滚动结果一致。

**F2：关闭**

`src/style.css:222–233` 使用 `clamp(2.5rem, 14vw, 6rem)` 和移动 Ely 的 `right: 0`。320×568 下标题右边界为 296.61px，Ely 右边界为 320px；390×844 下分别为 357.11px 和 390px。两组产品名可辨识，横向裁切已消除。

六组独立测量均满足 `clientWidth === scrollWidth === innerWidth === configuredWidth`：

| Viewport | clientWidth / scrollWidth / innerWidth | Title right | Ely right | Title bottom / terminal top |
|---|---|---:|---:|---|
| 1440×900 | 1440 / 1440 / 1440 | 776.05 | 1353.61 | 352.22 / 387.00 |
| 1280×720 | 1280 / 1280 / 1280 | 689.81 | 1203.20 | 303.48 / 307.20 |
| 390×844 | 390 / 390 / 390 | 357.11 | 390.00 | 225.44 / 413.58 |
| 320×568 | 320 / 320 / 320 | 296.61 | 320.00 | 168.80 / 366.97 |
| 844×390 | 844 / 844 / 844 | 517.33 | 818.69 | 260.59 / 352.41 |
| 1280×600 | 1280 / 1280 / 1280 | 689.81 | 1203.20 | 313.09 / 344.00 |

`tests/visual.mjs:32,54–70` 改用实际视口截图，比较 scrollWidth 与 clientWidth、innerWidth 与配置宽度，并检查标题/Ely 右边界、标题与终端的纵向关系及短屏实际滚动。原测试的横向溢出漏报路径已封住。六张初始 PNG 的像素尺寸均与配置视口一致。

**验证结果**

| Check | Result |
|---|---|
| 隔离构建 | 仓库外临时副本执行 `npm ci`、`npm run build`，均退出 0。JS 为 523.42 kB / gzip 130.28 kB；保留已知尺寸警告，不构成本轮阻塞。 |
| 原视觉测试 | 对新的生产构建运行现有 `tests/visual.mjs`，六组全部通过，failures 和 page errors 均为空。 |
| 独立行为复验 | Chrome 156.0.8078.48；六组逐一检查配置宽度、标题/Ely 边界、标题与终端关系、导航内容边界；三种字体均 loaded。短屏滚轮终点检查通过。 |
| 导航 | 六组均通过 Tab 聚焦 skip link、Enter 到 #content、Terminal 到 #terminal、GitHub 点击到精确仓库 URL。GitHub 请求由测试拦截，仅验证目标与浏览器导航行为。 |
| 内容与任务边界 | 内容保持产品名、导航和终端应用预览；未发现嵌套营销卡片、tagline 或 filler copy。最终姿态、synthwave 背景、其他段落、下载与 locale/theme controls 继续归 W4–W8。 |

独立探针初次将页眉整体盒子的空白内边距纳入遮挡判断；改按导航链接实际边界测量后，六组通过。该探针修正仅发生在临时运行命令中。

**输入标识**

| Input | SHA-256 |
|---|---|
| `website/index.html` | `575ef2048e935cc8fc61db5e7ad065c813aefe1361348941eeebe6b9b96430b2` |
| `website/src/style.css` | `96d173d6d2644408dd3f292c411c799e8c4560aa108bca77bb625fc08fac71f4` |
| `website/src/terminal.css` | `f66b3efee62c5cd699133f3c139408ea94df8d7a4e189511c6774ed2756b4659` |
| `website/src/main.js` | `dd6f5bee9c7a09c0ccef4db1ef727aae5f60800db4baca81b99d852b03d0a73b` |
| `website/tests/visual.mjs` | `7f00fe230bfdf4d3318a24214deadaaa5b94d04069a8fd93f50da9880412f81b` |

构建、测试和新截图均在仓库外临时副本完成。浏览器与预览服务器已关闭，临时副本已清理；文件哈希核对确认本轮仓库写入范围仅为 `website/reviews/W3-codex-r2.md`，原始九张截图保持原状。

未测试 Safari、Firefox、实体手机或生产部署；移动端结论来自 Chrome 手机仿真。当前 W3 范围未发现剩余阻塞。

