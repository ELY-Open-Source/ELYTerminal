FAIL

W12 第 2/3 轮。第一轮在场景入口的三条重叠路径已修复；320px 页面继续向下阅读时，Ely 仍与正文重叠，F1 保留。

**F1：跨槽位插值把 Ely 移出预留空白，阅读正文时持续叠字。**

位置：`website/src/stage.js:231`–`:237`、`website/src/ely.js:33`–`:49`；验证遗漏位于 `website/tests/content.mjs:69`–`:79`。CSS 槽位本身已避开文字，但每帧把两个场景槽位的实时矩形按整段滚动进度插值。进入场景后继续阅读，实际人物就离开当前槽位，穿过仍然可见的标题、会话或 Git 内容。新增测试比较文字与空槽矩形，实际插值后的形体位置没有进入断言。

复现：使用 320×568、英文深色、正常动画；等待开场完成，将 Sessions 或 Git 滚至场景顶部，再通过原生鼠标滚轮向下滚动 400px，停止滚动并等待 3 秒。两个路径分别停在 `scrollY=1038` 与 `scrollY=1821`，当前 `data-scene` 仍分别为 sessions 和 git。

| 路径 | 实际结果与证据 |
| --- | --- |
| Sessions 顶部再下滚 400px | Ely 与 `~/Projects/alacritty`、`~/Documents` 和 agent 转录相交；对应文字行框内分别有 604、501、131 个有效 Ely 单元中心。[原生滚轮截图](../../target/website-review-w11-r2/run-ecqs2_0x/shots/native-sessions-400.png) |
| Git 顶部再下滚 400px | Ely 与已暂存文件、diff 命令、文件统计行及汇总相交；`src/term/view.rs` 统计行内有 347 个单元中心，汇总内有 65 个。[正常动画截图](../../target/website-review-w11-r2/run-ecqs2_0x/shots/scroll-no-preference-git-400.png) |
| 同一 Git 位置切换中文、浅色 | 重叠继续存在。[中文浅色截图](../../target/website-review-w11-r2/run-ecqs2_0x/shots/native-zh-light-git-400.png) |
| 初始 reduce 重放阅读路径 | 同样复现，停止滚动后的静态人物仍与正文相交。[减弱动画截图](../../target/website-review-w11-r2/run-ecqs2_0x/shots/scroll-reduce-git-400.png) |

测量只计实际 WASM 输出中属于成形 Ely 的非空单元，排除环境尘埃；同时检查可见文字 Range。上述 Git 路径的 CSS 槽位已在视口上方约 -317 至 -147px，4854 个有效人物单元全部位于该槽外，人物仍画在正文区域。停止滚动 3 秒后的截图与数据确认这是稳定布局重叠。横向溢出仍为 0。

影响：手机用户滚动阅读下方会话和 diff stat 时，产品文字与人物字形叠印；入口空槽检查无法代表整个阅读路径。

最小修复：保留空槽布局，修正场景内滚动与跨场景移位的进度或轨迹，使实际人物在正文可见期间仍位于预留空白，并保持连续移位。回归必须覆盖上述中间滚动位置，比较实际绘制形体与可见文字；补充入口、页面底部和同页缩放检查。

证据：[scroll.json](../../target/website-review-w11-r2/run-ecqs2_0x/scroll.json)、[native.json](../../target/website-review-w11-r2/run-ecqs2_0x/native.json)；脚本为 [scroll.mjs](../../target/website-review-w11-r2/run-ecqs2_0x/probes/scroll.mjs) 与 [native.mjs](../../target/website-review-w11-r2/run-ecqs2_0x/probes/native.mjs)。

**已关闭路径与回归结果。**

六视口 × 英文/简中 × 深/浅主题共 24 组，四场景入口及 Files 页签的独立检查全部通过：实际成形粒子均在对应槽位内，人物与文字相交数为 0。320×568 Sessions/Git 与 844×390 Download 的原入口路径，在正常动画下再次通过。已目视检查新增窄屏、短屏截图、深浅主题场景与独立复现截图。

844×390 实际滚至页面底部后，标题、主下载链接和六个格式链接可见，人物与文字相交数为 0，见 [底部截图](../../target/website-review-w11-r2/run-ecqs2_0x/shots/download-844-bottom.png)。1280×720 同页缩至 1280×600 后滚到底部，人物仍处于 Download 槽位内且没有叠字，见 [缩放截图](../../target/website-review-w11-r2/run-ecqs2_0x/shots/download-resize-bottom.png)。

在 target 临时副本执行干净安装、单元测试（33/33）、生产构建，以及 preferences/downloads/content/stage/visual 五套浏览器测试，全部通过；构建仅有已接受的 Three chunk 警告，stage 平均帧间隔 16.67ms。键盘页签、语言/主题与持久化、下载映射、存储拒绝及静态回退回归通过；普通页面未记录 pageerror 或 console.error。

检查的相关手写文件均不超过 500 行，最大为 `style.css` 的 485 行；注释均为一行且不超过十词。旧 `places()` 和未用 RAMP 导入已删除；本次修复未引入营销嵌套卡片、额外文案或旧玻璃样式。W11 初始化期间的偏好问题单列于 [W11 本轮报告](W11-codex-r2.md)。

命令日志、测量与截图保存在 [本轮目录](../../target/website-review-w11-r2/run-ecqs2_0x/)。临时构建目录、安装依赖与缓存已删除；既有 746 个文件、87 张截图及原 WASM 哈希保持一致。仓库仅新增本轮两份报告。

验证边界：本机 Chrome 156.0.8078.48 与视口/媒体模拟；未运行 Safari、Firefox、实体触屏设备或实际读屏软件。始终使用已有开发服务器，未访问生产站点。
