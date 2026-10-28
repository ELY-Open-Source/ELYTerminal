FAIL

W7 第 2/3 轮复审，2026-10-28。F1 已关闭。F2 在直接加载短屏页面时通过，窗口调整尺寸后仍可复现重叠，剩余阻塞为这一条 F2 路径。

**F2 — 视口高度变化未触发姿态重算，短屏下载布局再次重叠**

文件：`website/src/ely.js:103–106`、`:132–134`；短屏选择位于 `:57`、`:67`。

根因：短屏 Download 姿态在 `measure()` 中依据 innerHeight 生成并缓存。后续重测只依赖观察 main 元素的 ResizeObserver。1280×720 与 1280×600 下，main 的宽高都为 1280×2880px：前者四个场景各 720px，后者前三个场景各 704px、Download 为 768px。因此真实视口高度变化后，main 的尺寸没有变化，观察器不触发，缓存的居中 Download 姿态继续生效。

复现：在同一页面以 1280×720 打开 `http://127.0.0.1:5173`，滚至 Download 并等待姿态稳定；将窗口视口改为 1280×600，保持页面加载状态；在页面右侧滚轮向下 1000px 至底部，等待 1200ms。独立探针使用 Playwright 的 setViewportSize 改变视口，随后发送真实滚轮事件。

| 状态 | main 高度 | Ely transform | 结果 |
|---|---:|---|---|
| 1280×720 初始页面 | 2880px | 居中，约 translate(461px,72px) scale(1.06) | 标题与 Ely 分离。 |
| 同一页面缩至 1280×600 | 2880px | translate3d(461px,72px,0px) scale(1.06) | 旧姿态保留，Download 标题横穿 Ely 所持终端。 |
| 1280×600 重新加载对照 | 2880px | translate3d(25.6px,0px,0px) scale(0.8) | 短屏姿态生效，重叠消失。 |

失败状态的实际 scrollY 为 2280px。标题范围为 x=394.27–885.73、y=281–379.55；Ely 图形范围为 x=510.23–773.01、y=55.01–428.17，相交约 262.78×98.55px。已目视核对缩窗后的截图及重新加载对照。

影响：用户正常缩小浏览器窗口即可再次进入第 1 轮的下载标题重叠状态，仍阻碍“桌面下载布局不得重叠”的验收。

最小修复：视口 resize 时调用既有 measure()，更新 anchors 与 poses，并由现有 rAF 调度器应用结果；保留对内容尺寸变化的观察。为同一页面的 1280×720 → 1280×600 增加回归，调整视口后滚到底、等待姿态稳定，验证短屏 transform 和 Ely/标题/操作区分离。当前 downloads.mjs 为每个视口新建页面，覆盖了初始布局，未覆盖上述缓存更新路径。

证据均在 `target/website-review-w7-r2/run-nf05ud4x/`：`website/resize-probe.mjs`、`resize-probe.json`、`resize-720-to-600.png`、`resize-600-reloaded.png`。

**F1 关闭验证与 F2 已通过路径**

重新读取第 1 轮报告，使用同一 desktop Safari UA、MacIntel、userAgentData=undefined，分别注入 maxTouchPoints=5 和 0。独立函数及浏览器页面结果一致：前者返回 other，主链接为 releases 页面，标签为 View releases；后者返回 macos，主链接为稳定 DMG，标签为 Download for macOS。Windows/Linux、iPad 移动 UA、iPhone、Android、ChromeOS、FreeBSD 对照均符合要求。现有浏览器回归中的 macOS client hint + maxTouchPoints=5 也通过。

分别直接加载 844×390 和 1280×600，再用真实滚轮到达页面底部，新的左侧缩小姿态均生效。844×390 的 Ely 图形右边为 220.87px，标题左边为 259.97px；1280×600 的 Ely 图形右边为 305.03px，标题左边为 394.27px。两种尺寸的标题、主按钮均与 Ely 分离。已查看用户提供的两张 bottom 截图及独立生成的截图。F2 的剩余范围为上文的运行中尺寸变化。

**独立执行结果**

所有安装、构建、测试、浏览器临时目录及新截图均位于指定临时目录。浏览器连接原开发服务器。发布测试使用本地 cargo/gh 桩。

| Command | Result |
|---|---|
| `npm --prefix website test` | 退出 0；6/6 通过。 |
| `npm --prefix website run build` | 退出 0；14 个模块，JS 534.34 kB / gzip 134.46 kB；保留已接受的 Three chunk 警告。 |
| `node website/tests/visual.mjs` | 退出 0；六视口通过，字体 loaded，errors / failures 为空。 |
| `node website/tests/ely.mjs` | 退出 0；八张姿态捕获与减弱动画吸附通过。 |
| `node website/tests/content.mjs` | 退出 0；内容、页签与 W6 末尾可达性回归通过。 |
| `node website/tests/scene.mjs` | 退出 0；深浅场景、减弱动画及 WebGL 回退通过；平均帧间隔 16.67ms。 |
| `node website/tests/downloads.mjs` | 退出 0；五种平台输入、六链接、六视口及新增底部交叠断言通过。 |

干净 npm ci 通过。独立六视口检查继续验证标题及下载链接处于视口内、无横向溢出、链接相互分离且可命中；逐个点击时在浏览器内阻止导航，六个格式 href 与规定别名完全相同，page errors 为空。

发布脚本与第 1 轮逐字节相同。重新运行本地桩检查：六格式的普通上传/alias-only 共 12 个映射与 payload SHA-256 通过；四个参数/未知别名拒绝场景在上传前失败；三个上传故障场景正确非零退出并清理临时目录。公开上传命令未执行。

源码差异限于 downloads.js、ely.js 及两份下载测试。main.js、index.html、场景/终端 CSS、背景、页签模块和 release-asset.sh 与第 1 轮相同。修复没有新增营销嵌套卡片或文案；四个修改文件分别为 54、150、31、89 行，均无源码注释。既有六稳定别名、版本无关 URL 和单一发布流程保持不变。

**输入标识与审查边界**

| File | SHA-256 |
|---|---|
| `website/src/downloads.js` | `e8e3038add46ad9f09fea7091d88e5e5fddf228dbfac87e7349406be495571b0` |
| `website/src/ely.js` | `dc993264c69476f10fbbbe8dde6534da8786b2f88db30753d3e36dd82c899546` |
| `website/tests/downloads.test.js` | `0ae3c72bc95c5e4a7dde196f3bf7260dc7b83f3d3644bfa6d1c88d2ea24c472b` |
| `website/tests/downloads.mjs` | `d3af40d3c9fd3ec1f63a7fc859d61ce423ac6478b8a431098968a70ea24c3feb` |

除指定临时目录外，本轮仅新增本报告。起始文件和 40 张原截图的 SHA-256 已复核；实现、公开 release、远端资源及开发服务器均未由本轮修改，独立浏览器已关闭。

公开别名上传与 redirected HEAD 验证待所有者授权后进行，本轮未验。实体 iPad、原生 Safari/Firefox 和生产下载未测试；平台行为来自 Chrome 属性仿真，窗口尺寸变化与布局证据来自 Chrome 156.0.8078.48。
