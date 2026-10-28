FAIL

W11 审查，2026-10-28。依据当前 `TASKS.md` W11、`PRD.md` §1–4、§7 与本轮仓库约束。运行中改变减弱动画偏好会留下错误的运动状态，构成功能与可访问性阻塞。

**F1：字形群永久缓存首次加载时的减弱动画偏好。**

位置：`website/src/stage.js:67`、`:79`、`:153`、`:216`、`:222`、`:241`。`reduced` 是初始化时取得的布尔常量；绘制、阻尼、场景冲击与按压处理一直读取该常量，`uScramble` 也只设置一次。该模块没有监听媒体查询变化。

复现：在现有开发服务器以 1440×900、`reducedMotion: 'no-preference'` 打开页面，等待开场与 Ely 成形；同页调用 Playwright `page.emulateMedia({ reducedMotion: 'reduce' })`，保持页面静止，再移动指针并按下鼠标。浏览器媒体查询已经返回 `true`，实际字形群仍持续绘制：两次采样间 draw 次数从 302 增至 341，WASM 输出位置继续改变，两张画布截图不同。按压后仍有 755 个粒子的速度超过乱码阈值 160，冲击与乱码仍然启用。反向路径也复现：以 reduce 加载后改回 no-preference，draw 次数一直为 2，指针和按压继续保持静态。

影响：访客在页面打开期间启用系统减弱动画后，环境漂移、阻尼、冲击和乱码继续运行，违反 §4 的减弱动画行为。`type.js` 已实时响应同一偏好，导致 DOM 动画与字形群的状态分裂。

最小修复：保留 MediaQueryList 并处理其 `change`；同步当前运动状态与 `uScramble`，进入 reduce 时结束尚在进行的开场、清除指针作用并直接形成当前姿态，使静止页面停止环境运动；恢复完整动画时恢复相应路径。回归覆盖同页双向切换，检查像素稳定、按压和场景切换；现有 `tests/stage.mjs` 只在创建上下文时设置 reduce。

证据：[独立探针结果](../../target/website-review-w11/run-l4xr6w8b/review.json) 的 `motion`；[切换后第一帧](../../target/website-review-w11/run-l4xr6w8b/shots/reduced-no-preference-before.png)、[切换后第二帧](../../target/website-review-w11/run-l4xr6w8b/shots/reduced-no-preference-after.png)。可复查脚本为 [review.mjs](../../target/website-review-w11/run-l4xr6w8b/probes/review.mjs)。

**仓库规则检查。**

还有两处明确规则不符合：`website/src/stage.js:4` 导入 `RAMP`，该文件全文仅此一次出现，应删除未用导入；`website/src/ely.js:109` 的注释 `Pose a copy of the SVG, then read its glyph grid.` 为 11 个英文词，应删除或缩至十词以内。相关手写实现、测试和配置文件均未超过 500 行。以上为静态规则项，功能性 FAIL 依据是 F1。

**独立执行与通过的证据。**

在本轮临时副本执行干净安装、单元测试和构建；浏览器始终连接已有 `http://127.0.0.1:5173`。

| 检查 | 实测结果 |
| --- | --- |
| `npm --prefix website ci --no-audit --no-fund` | 通过 |
| `npm --prefix website test` | 33/33 通过，包含字形网格与实际编译 WASM 的物理测试 |
| `npm --prefix website run build` | 通过；仅保留已接受的 Three chunk 大小警告 |
| `node website/tests/{preferences,downloads,content,stage,visual}.mjs`，逐个执行 | 五套全部通过；stage 平均帧间隔 16.67ms |
| 独立浏览器探针 | Chrome 156.0.8078.48，Metal；六视口 × 双语言 × 双主题，四场景逐一检查 |

临时重新编译、原工作区以及服务器实际提供的 WASM 的 SHA-256 均为 `1271679bc375497334cd92bb6ec6d704bbbd7409a988a79cce05d2a7afc670d7`。这将编译后物理测试与浏览器运行的二进制对应起来，记录见 [wasm-hashes.json](../../target/website-review-w11/run-l4xr6w8b/wasm-hashes.json)。

已目视检查开场落字、拼出 ELYTerminal、飞入 Ely 的连续截图，以及深浅主题四场景。WebGL 探针记录桌面每次实例绘制 10080 个实例、窄屏 8180 个实例；源码使用单个实例网格，姿态来自原 SVG 的离屏采样，填充与方向轮廓字形有单元覆盖。完整动画下指针排斥与按压冲击均发生，四个场景有不同成形结果和对应日志。

原生滚轮 900px 后实际 `scrollY=900` 并进入 Sessions；PageDown 后 `scrollY=1760` 并进入 Git。初始 reduce 路径跳过开场且静止像素稳定。1280×720 同页调整为 1280×600 后画布尺寸同步正确。普通路径未记录 pageerror 或 console.error。

分别注入 WASM 404、WebGL 创建失败和真实 `WEBGL_lose_context`：均显示静态原画、恢复标题、保留整页内容，输出带 ELYTerminal 所属信息的明确错误日志；320px 回退页面无横向溢出。故障日志属于故障注入的预期结果。

页面文字与 Ely 的窄屏重叠归入本轮 [W12 报告](W12-codex.md)。命令日志、矩阵测量、原生滚动与截图保存在 [本轮证据目录](../../target/website-review-w11/run-l4xr6w8b/)；临时构建、安装依赖与缓存已删除。哈希复核确认既有 744 个文件、71 张截图及原 WASM 均未改变。

验证边界：浏览器覆盖为本机 Chrome 与视口/媒体模拟；未运行 Safari、Firefox、实体触屏设备或实际读屏软件。未访问生产站点，下载核验限本地 URL 与平台映射。
