FAIL

W12 审查，2026-10-28。依据当前 `TASKS.md` W12、`PRD.md` §1–4、§7 与本轮仓库约束。平面页面在 320px 与横向短屏存在可稳定复现的正文/下载区与 Ely 字形重叠。

**F1：布局未为窄屏和短屏的成形 Ely 保留文字空间。**

位置：`website/src/ely.js:48`、`:49`、`:57`；`website/src/style.css:169`、`:188`、`:405`、`:410`。移动端 Sessions/Git 的 Ely 固定以 `0.8 × viewport height` 为中心、高度为 `0.34 × viewport height`；文字按自身内容排布并在 320px 换行。桌面 Download 的 Ely 固定居中且高为 `0.56 × viewport height`，下载内容同时靠场景底部排列。两组定位都没有协调内容实际占用的空间。

复现：打开已有开发服务器，等待字体与开场结束，把目标场景滚到页面顶部，再等待 3.5 秒使字形和转录完全稳定。以下三个正常动画路径均独立复现；初始 reduce 下再次复现。英文/中文、深色/浅色矩阵均出现相同问题。

| 视口与场景 | 实际结果 | 截图 |
| --- | --- | --- |
| 320×568，Sessions，`scrollY=658` | 最后一个会话的路径及 agent 转录与 Ely 头部/身体叠印；`Reading src/term/view.rs`、`Checking input and resize paths`、状态行穿过字形图案 | [正常动画](../../target/website-review-w11/run-l4xr6w8b/shots/full-320x568-sessions.png) |
| 320×568，Git，`scrollY=1233` | diff stat 的文件行、增删符号及汇总穿过 Ely 头部和身体 | [正常动画](../../target/website-review-w11/run-l4xr6w8b/shots/full-320x568-git.png)、[中文浅色](../../target/website-review-w11/run-l4xr6w8b/shots/zh-CN-light-320x568-git.png) |
| 844×390，Download，页面底部 `scrollY=1569` | Download 标题和主下载链接与 Ely 身体重叠 | [正常动画](../../target/website-review-w11/run-l4xr6w8b/shots/full-844x390-download.png) |

测量读取实际 WASM 输出，只统计成形 Ely 的非空网格粒子，并与 DOM 文字 Range 相交。320px Sessions 两条转录的文字矩形内分别落入 312、441 个 Ely 单元中心；Git 汇总文字内有 491 个；844px Download 标题与主链接内分别有 1092、245 个。空网格和环境尘埃已排除。截图确认字符图案与正文直接重叠，影响转录、diff stat 和下载操作区的视觉辨识。三个路径的横向溢出均为 0，说明仅检查 scrollWidth 无法发现该缺陷。

最小修复：调整窄屏 Sessions/Git 及短屏 Download 的人物位置、尺寸或内容预留空间，使稳定姿态与正文、标题和操作区分开。回归加入上述三个路径，等待字体、转录和人物成形后检查实际可见人物与文字的相交；继续覆盖双语言、双主题和 reduce。当前 `tests/content.mjs` 只检查 DOM 内容边界，`tests/visual.mjs` 截图矩阵只有 1440×900、1280×720、390×844，均未检查这些文字与 WebGL 图案的相交。

证据：[review.json](../../target/website-review-w11/run-l4xr6w8b/review.json) 的 `matrix`、[extra.json](../../target/website-review-w11/run-l4xr6w8b/extra.json) 的 `full`；独立脚本为 [review.mjs](../../target/website-review-w11/run-l4xr6w8b/probes/review.mjs) 和 [extra.mjs](../../target/website-review-w11/run-l4xr6w8b/probes/extra.mjs)。

**独立执行与已核验部分。**

干净安装、`npm --prefix website test`（33/33）、生产构建，以及 `preferences.mjs`、`downloads.mjs`、`content.mjs`、`stage.mjs`、`visual.mjs` 五套浏览器测试全部通过。构建仅保留已接受的 Three chunk 大小警告。命令在临时副本运行，全部浏览器访问已有 `http://127.0.0.1:5173`。

独立矩阵覆盖 1440×900、1280×720、390×844、320×568、844×390、1280×600，各自英文/简中与深/浅主题，共 24 组；每组检查四场景，并操作 Git/Files 页签。矩阵无横向溢出和普通路径控制台错误。已逐张检查深浅桌面四场景、移动端场景与 Files，以及上述缺陷截图；390×844 正常场景的文字与人物位置分开。

页面使用本地 JetBrains Mono 与平面文本结构，旧玻璃应用窗、Aurora/dither 实现、Space Grotesk 资源和样式引用已清除。未发现新增营销嵌套卡片、tagline、描述、装饰 badge 或二级标签。隐藏的原 SVG 用于姿态采样与可见静态回退，属于当前实现的有效路径。

anime.js 的标题解码、转录逐字/逐行显示、五种 3×3 会话状态具有对应实现与单元覆盖；截图捕获于动画完成后，正文保持完整。解码期间提供完整可访问名称，状态有本地化文本。Git/Files 鼠标切换、方向键、Home/End 与 ARIA 状态通过；额外独立执行了键盘页签、Enter 切换语言、Space 切换主题以及原生滚轮/PageDown。

语言/主题初始选择、保存与重载、系统外观变化、44px 控制目标、320px 焦点轮廓与 localStorage 被拒绝的路径通过现有浏览器测试。六个稳定下载 URL、平台主链接及不支持平台的 releases 链接通过单元与浏览器核验。普通页面中未记录 pageerror 或 console.error。W11 的运行中减弱动画缺陷单独记录在 [W11 报告](W11-codex.md)。

W12 手写实现、测试与配置文件均不超过 500 行；检查的 W12 注释均为单行、十词以内，未发现额外死代码。加载、字形场景成形及失败路径有明确日志；故障注入后静态 Ely 与整页内容可见。

证据与截图保存在 [本轮目录](../../target/website-review-w11/run-l4xr6w8b/)。临时构建、安装依赖与缓存已删除；既有 744 个文件、71 张截图及原 WASM 的哈希均保持一致。仓库新增内容仅为本轮两份报告。

验证边界：本机 Chrome 156.0.8078.48，桌面和移动视口模拟；未运行 Safari、Firefox、实体触屏设备或实际读屏软件。未访问生产站点，也未重新请求公开 release 资产。
