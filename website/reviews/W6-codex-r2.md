PASS

W6 第 2/3 轮复审，2026-10-28。第 1 轮 F1 已关闭；本次修复未发现可复现的新增阻塞。

**F1 根因与关闭证据**

已重读第 1 轮报告，核对修复源码及新增回归，并与第 1 轮保存的源码副本比较。实现改动集中在 `terminal.css`、`scenes.css`；`index.html`、`main.js`、`scenes.js`、`style.css`、`ely.js` 和 `scene.js` 与上一轮相同。

`terminal.css:79–94` 通过 `grid-template-rows: minmax(0, 1fr)` 将网格行限制在工作区高度内，并让会话栏及 Git 栏以 `min-height: 0` 收缩、通过 `overflow-y: auto` 访问溢出内容。`scenes.css:31–32` 保持场景 Git 外栏裁剪；`:60–65` 将实际滚动交给内部 tab panel。Files 继承该滚动区域。上一轮“内容撑高父级、外框裁剪、内部 scrollTop 始终为 0”的因果链已经解除。

独立探针使用 Chrome 156.0.8078.48，连接 `http://127.0.0.1:5173`，等待字体加载。320×568 使用手机仿真，1280×720 使用桌面视口。每次先滚至对应场景，在窗格内定位指针，通过真实滚轮向下 160px；探针未给窗格赋值 scrollTop。

| 第 1 轮路径 | clientHeight / scrollHeight | 实际滚轮后 scrollTop | 末尾可达结果 |
|---|---|---:|---|
| 320×568 Sessions | 312 / 361px | 49px | nvim 整行及 idle 标记可见；行底边 492.70px，滚动区域底边 511.03px。 |
| 320×568 Git | 248 / 265px | 17px | 最近提交正文可见；正文底边 497.28px，滚动区域底边 497.44px。 |
| 320×568 Files | 248 / 330px | 82px | tests、Cargo.toml、PRD.md 均可见；PRD.md 可命中，截图中文字完整。 |
| 1280×720 Files | 293 / 330px | 37px | PRD.md 可见；行底边 634.36px，滚动区域底边 634.39px。 |

四条路径均验证了目标与全部裁剪祖先的边界、中心点命中及截图；行盒几何比较使用 1px 的亚像素舍入容差。Sessions 的 idle 标记也独立核验。向下滚动期间页面 scrollY 保持原值，实际移动发生在相应窗格；反向滚轮后四个窗格均回到 scrollTop=0。

已目视检查四条路径的末尾截图和 320×568 Git 顶部截图。Git/Files 页签在内容滚动后保持可操作；独立复验点击、ArrowLeft/ArrowRight、Home/End、焦点及 aria-selected 状态均通过。两种独立视口的页面宽度始终满足 `scrollWidth === clientWidth === innerWidth === configuredWidth`，page errors 为空。

**指定命令与回归**

六项命令均独立执行。为遵守文件写入范围，安装、测试、构建和新截图使用仓库外副本；浏览器脚本连接指定开发服务器。已核对服务端 HTML 与当前源码一致，差异仅来自 Vite 客户端脚本及模块时间戳。

| Command | Result |
|---|---|
| `npm --prefix website test` | 退出 0；2 个测试通过。 |
| `npm --prefix website run build` | 退出 0；13 个模块，JS 532.85 kB / gzip 133.76 kB。保留已接受的 Three chunk 尺寸警告。 |
| `node website/tests/visual.mjs` | 退出 0；六视口通过，字体 loaded，errors / failures 为空。 |
| `node website/tests/ely.mjs` | 退出 0；八张姿态捕获及减弱动画吸附通过。 |
| `node website/tests/content.mjs` | 退出 0；六视口、双语数据、页签，以及新增的末尾可见性和实际溢出断言通过。 |
| `node website/tests/scene.mjs` | 退出 0；深浅配色、减弱动画、WebGL 回退通过；帧间隔均值 16.67ms。 |

额外执行干净 `npm ci`，退出 0。六视口为 1440×900、1280×720、390×844、320×568、844×390、1280×600。

`content.mjs:13–26` 的新辅助函数用于测量实际滚动范围和末尾位置；调用覆盖最后会话行、idle 标记、最近提交正文及 Files 的 PRD.md，并对指定窄屏/笔记本视口断言 scrollHeight 大于 clientHeight、scrollTop 大于 0。独立真实滚轮复验补充确认了用户操作路径。

共享样式相关的 hero、内容及场景回归通过；另外目视检查了重新生成的笔记本 hero、横屏 hero、390px Git 和浅色 Git 截图。当前修复保持既有内容结构与文案，营销嵌套卡片和无用文案约束继续满足。修改文件均低于 500 行：`terminal.css` 385 行、`scenes.css` 237 行、`content.mjs` 107 行；这三份文件没有源码注释，新增函数和样式均有使用。

**输入标识与写入边界**

| File | SHA-256 |
|---|---|
| `website/src/terminal.css` | `40e24e19195e9bae40c70d79676834ad9990f92dcb51e6733dd50eac515cb59d` |
| `website/src/scenes.css` | `8a613466b0589211a7fe63b2f410517cdc67bb49528d99fc862d510085eecf4d` |
| `website/tests/content.mjs` | `a1a7262a8c2a66ebb852691dc5d102cd0f0f02ba24b45fd264bf6b961fc1da64` |

本轮仅新增本报告；起始文件及 35 张原截图的 SHA-256 已复核。开发服务器保持运行，独立浏览器已关闭。命令日志、探针和新截图位于仓库外 `/var/folders/nj/z1qswzwd75vcdmnmpn31w1jc0000gn/T/ely-w6-r2-review-8bsdxpcv/`。

本轮范围为 F1 关闭及修复引入的阻塞；W7/W8 仍由后续任务处理。未测试 Safari、Firefox、实体手机和生产部署；手机结论来自本机 Chrome 仿真。
