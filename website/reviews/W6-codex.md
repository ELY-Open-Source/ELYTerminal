FAIL

W6 第 1/3 轮审查，2026-10-28。发现 1 个可复现阻塞：受限高度的应用窗口裁掉场景内容，用户滚动无法访问被裁部分。

**F1 — 窗格撑开网格行后被应用窗口裁剪**

位置：`website/src/scenes.css:19`、`:56`、`:67`、`:184`，以及复用的 `website/src/terminal.css:12`、`:79`、`:86`、`:234`。

根因：场景窗口使用限定高度，外层 `.app-window` 为 `overflow: hidden`。内部 `.workspace` 的隐式网格行仍按内容最小尺寸扩展；作为网格项的 `.session-rail` 和 `.git-panel` 保留 `min-height: auto`。窗格因此超过窗口的可用高度。Files 虽然声明 `overflow: auto`，其父级已经被内容撑高，实际 `clientHeight === scrollHeight === 330`，没有可滚动的溢出区域。Sessions 和 Git 内容也缺少能访问被裁部分的滚动区域。

复现环境：Chrome 156.0.8078.48，`http://127.0.0.1:5173`，字体加载完成。320×568 使用手机仿真；1280×720 使用桌面视口。将页面滚至对应 section 的 `offsetTop`，Git 场景通过实际点击 Files 页签切换。

| 视口 / 内容 | 应用窗口底边 | 被裁元素位置 | 实际结果 |
|---|---:|---|---|
| 320×568 / Sessions | 512.03px | 最后一行 nvim 为 486.92–541.70px；idle 标记为 506.31–522.31px | idle 标记大部分及目录文字被裁。 |
| 320×568 / Git | 512.03px | 最近提交正文为 515.28–528.28px | `Refine the terminal view` 整行被裁。 |
| 320×568 / Files | 512.03px | `PRD.md` 为 559.33–579.98px | 文件树末尾的 tests、Cargo.toml、PRD.md 隐藏，assets 行也被截断。 |
| 1280×720 / Files | 648.98px | `PRD.md` 为 650.70–671.36px | 最后一行完全被裁。 |

在上述窗格内实际滚轮向下 120px，页面发生原生滚动；内容相对窗口的裁剪量保持相同。随后将对应窗格的 `scrollTop` 设为 10000，读回仍为 0。滚动后的被裁文字中心命中场景背景。独立截图也显示窗口底边直接截断上述内容，页面继续向下滚动仍无法露出它们。

最短复验：打开 1280×720 视口，滚至 Git，点击 Files，检查窗口底部的 `PRD.md`。控制台可执行以下只读布局检查；在当前实现中，最后一行顶部已经超过窗口底边。

```js
const app = document.querySelector('#git .app-window');
const panel = document.querySelector('#files-panel');
const last = panel.querySelector('.file-tree > li:last-child');
console.table({
  appBottom: app.getBoundingClientRect().bottom,
  lastTop: last.getBoundingClientRect().top,
  lastBottom: last.getBoundingClientRect().bottom,
  clientHeight: panel.clientHeight,
  scrollHeight: panel.scrollHeight,
});
```

修复要求：让 Sessions、Git、Files 的内部布局遵守应用窗口可用高度，并保证全部内容能通过正常页面流或可操作的内部滚动访问。验收必须覆盖 320×568 的最后会话及 idle 标记、Git 最近提交正文，以及 320×568 和 1280×720 的 Files 最后一行。

`tests/content.mjs:25–42` 只比较标题、外框与 section 边界；`:47–58` 检查面板本身可见及页签切换。内部内容被祖先裁剪时，这些断言仍通过。为 F1 增加回归断言，检查最后一项相对裁剪祖先的实际可见范围，并验证用户滚动能到达末尾。

**独立执行结果**

为保持仓库写入范围，使用仓库外源码副本执行安装、测试和构建；浏览器脚本连接指定开发服务器，新截图写入临时副本。已核对服务器 HTML：差异仅为 Vite 注入的客户端脚本及模块时间戳。

| Command | Result |
|---|---|
| `npm --prefix website test` | 退出 0，2 个测试通过。 |
| `npm --prefix website run build` | 退出 0，Vite 8.3.5，13 个模块；JS 532.85 kB / gzip 133.76 kB。已接受的 Three chunk 警告保留。 |
| `node website/tests/visual.mjs` | 退出 0，六视口全部通过；字体 loaded，page errors / failures 为空。 |
| `node website/tests/ely.mjs` | 退出 0，八张姿态捕获及减弱动画吸附通过。 |
| `node website/tests/content.mjs` | 退出 0，六视口、双语数据及现有页签断言通过；覆盖缺口见 F1。 |
| `node website/tests/scene.mjs` | 退出 0，深浅场景、减弱动画和 WebGL 回退通过；90 个帧间隔均值 16.67ms。 |

额外执行干净 `npm ci`，退出 0。所有指定命令均独立执行。

**其余 W6 核验**

已读取全部指定源码、`tests/content.mjs` 和相关回归，对照 PRD §1–3 及 W6 条目。页面顺序为 Terminal → Sessions/live state → Git/Files → Download。Sessions 展示 working、waiting、done、error、idle 五种状态；新增动画在 reduced motion 下全部为 none。Git/Files 展示实际产品结构，窗口中的会话栏、终端和右栏属于功能窗格。当前内容未发现 tagline、无用描述、二级营销标签、装饰 badge 或营销大框套小框。

独立探针对六个视口逐一验证鼠标切换、ArrowLeft/ArrowRight 双向循环、Home/End、Enter/Space、焦点移动、单个 tabIndex=0、aria-selected、aria-controls、tabpanel 及 aria-labelledby 对应关系、隐藏面板状态，均通过。新增可翻译界面文字包含英文和简体中文数据；程序输出、路径、文件名和示例提交内容按产品内容处理。

六视口为 1440×900、1280×720、390×844、320×568、844×390、1280×600。独立检查中 `scrollWidth === clientWidth === innerWidth === configuredWidth`；Sessions/Git 标题与应用窗口边界均在视口宽度内，标题底边位于应用顶边上方。短屏原生滚轮能够移动页面。内部内容可达性的失败单独记录为 F1。

已目视查看仓库提供的六张 `content-*.png`，以及本轮重新生成的八张深浅 `scene-*.png` 和两张 WebGL 回退图；另外检查了独立探针生成的 320×568 Sessions/Git/Files 与 1280×720 Files 裁剪截图。截图与布局测量相符。独立探针的 page errors 为空。

当前范围未发现死代码或版本化 API；应用继续共用一个 rAF 调度器。指定手写源码和回归文件均低于 500 行，最长为 `terminal.css` 的 382 行；未发现源码注释。

**审查输入及文件边界**

| Input | SHA-256 |
|---|---|
| `website/index.html` | `431f5dc9cd21fe5ba8237b95ebf5b6e192b545fc99f60a79ddceecdf6fa9abee` |
| `website/src/scenes.css` | `7a87d94d4ab0d39618d6c5027dd0839a3110ed51b3c47be6937b4e345098a6b7` |
| `website/src/scenes.js` | `ab24586e55267bbef65ef8615b2022e9d0b990365f6ae6240af44b78ab107da5` |
| `website/src/terminal.css` | `d2226c324058a19e65dd2b5744b445b62c3e71ce927abac6da7944a9341d4e4c` |
| `website/tests/content.mjs` | `11e848712f1fa81c45b9a958199e6b434acf5d9bc59d0762571b617b52c6557a` |

本轮唯一仓库写入为本报告。审查前记录的文件与原始截图 SHA-256 已复核；开发服务器保持运行。独立浏览器均已关闭。临时探针及截图保存在仓库外 `/var/folders/nj/z1qswzwd75vcdmnmpn31w1jc0000gn/T/ely-w6-review-e2zi3x84/`。

W7 下载、W8 全站语言/主题控件与持久化按后续任务处理。未测试 Safari、Firefox、实体手机及生产部署；本轮手机结论来自 Chrome 仿真。阻塞修复范围为 F1 的内容可达性。
