FAIL

W11 第 3/3 轮。r2 的 F1 已关闭；回退截图复核确认一项实际可读性阻塞 F2。F2 在 r2 留存截图中已经存在，属于此前漏检，本次偏好修复没有引入它。

**F1 已关闭：加载期间的动画偏好与最终初始化状态一致。**

`website/src/stage.js:77` 完成 `createGlyphs` 后，`:79` 才读取 `motion.matches`；到 `:259` 注册监听之间全为同步执行。`uScramble`、开场分支和后续绘制使用同一偏好。

独立重放 r2 原路径：仅暂缓浏览器对本地 `swarm.wasm` 的请求，同页切换媒体偏好，等待 150ms 后放行。读取真实 WASM 输出、WebGL uniform、日志，并间隔 500ms 比较画布。

| 加载期间切换 | 当前结果 |
| --- | --- |
| full → reduce | 日志为 `stage ready, reduced motion`；`uScramble=0`；draw 为 2 → 2；4777 个 Ely 单元成形，物理位置和像素固定，跳过开场 |
| reduce → full | 日志为 `stage ready, full motion`，执行开场；`uScramble=1`；draw 为 276 → 315；物理位置和画布像素变化 |
| 成形后双向切换 | reduce 后像素固定，按压产生的高速粒子数为 0；恢复 full 后运动继续，按压产生 950 个高速粒子 |
| 开场中途启用 reduce | 开场结束并形成 Ely；等待标题过渡完成后，draw 为 4 → 4，连续 2.4 秒像素固定 |

证据：[probe.json](../../target/website-review-w11-r3/run-vqll668d/probe.json)、[独立探针](../../target/website-review-w11-r3/run-vqll668d/probes/probe.mjs)。

**F2：context lost 后保留的失效画布覆盖深色页面，破坏回退文字的可读性。**

位置：`website/src/stage.js:134`、`:267`；`website/src/style.css:107`、`:113`、`:405`、`:413`。

根因：`fail()` 停止更新并设置 `data-webgl='unavailable'`，CSS 显示静态 SVG、隐藏槽位，失效的全视口 WebGL canvas 继续保持 `display:block`。本机 Chrome 156.0.8078.48、Metal 后端将该 canvas 绘为白色占位，页面仍使用深色主题的浅色文字。

复现仅使用已有开发服务器，页面源码保持原样：

1. 用 320×568、深色、reduce 打开页面，等待 `data-swarm='ready'`。
2. 对 `#stage canvas` 的真实 WebGL2 context 调用 `getExtension('WEBGL_lose_context').loseContext()`，等待 2 秒。
3. 截取当前视口：背景变白，左上出现失效画布标记；导航、`Terminal` 标题部分和转录的浅色文字难以辨认，静态 Ely 仍可见。

实测 `data-webgl='unavailable'`、canvas 仍为 `block`；空白点 `(300,60)` 为 RGB `(255,255,255)`。主题正文色 `#e9e6e1` 对该背景的对比度约 1.24:1。在浏览器 DOM 中临时隐藏 `#stage` 后，同一点恢复 `(12,12,13)`，正文对比度约 15.71:1，静态 Ely、标题和转录正常显示。此对照仅改变浏览器内的画布可见性，仓库文件保持原样。

影响：WebGL 运行中丢失 context 时，静态回退虽已出现，页面文字仍被失效画布破坏。现有回退测试检查 SVG 可见和日志，未覆盖此文字/背景结果。

最小修复：在失败路径隐藏或移除失效 canvas，保留静态 SVG 和原主题页面；为真实 context lost 加入深色回退截图或像素检查，确认画布退出合成且正文可读。

证据：[失败视口](../../target/website-review-w11-r3/run-vqll668d/shots/lost-viewport.png)、[仅隐藏失效画布的对照](../../target/website-review-w11-r3/run-vqll668d/shots/lost-canvas-hidden.png)、[像素与状态](../../target/website-review-w11-r3/run-vqll668d/fallback.json)、[复现脚本](../../target/website-review-w11-r3/run-vqll668d/probes/fallback.mjs)。[r2 的同类截图](../../target/website-review-w11-r2/run-ecqs2_0x/shots/fallback-lost.png) 已有相同现象。

**其余验证。**

在 target 临时副本独立执行干净 `npm ci`、`npm --prefix website test`（32/32）、生产构建，以及 preferences、downloads、content、placement、stage、visual 六套浏览器测试，全部通过。构建仅保留已接受的 Three chunk 大小警告；stage 平均帧间隔 16.67ms。命令及输出见 [commands.log](../../target/website-review-w11-r3/run-vqll668d/commands.log)。

重新编译、工作区和服务器返回的 WASM 哈希一致。六视口 × 两语言 × 两主题共 24 组、120 个场景/页签采样通过；正常路径没有 pageerror 或 console.error。WASM 404、WebGL 初始化失败均显示静态 Ely并记录明确错误；context lost 的视觉问题单列于 F2。

检查范围内的手写实现与测试均不超过 500 行，最大为 `style.css` 的 485 行；注释均为单行且不超过十词。未用 RAMP 导入和 `placeAt` 已删除，未发现本次修复新增死代码或版本化 API。

证据保存在 [本轮目录](../../target/website-review-w11-r3/run-vqll668d/)。临时构建目录、安装依赖和缓存已删除；仓库仅新增本轮两份报告，既有 749 个文件、87 张截图及原 WASM 保持一致，见 [integrity.json](../../target/website-review-w11-r3/run-vqll668d/integrity.json)。

验证边界：本机 Chrome 与视口、媒体模拟；Safari、Firefox、实体设备和实际读屏软件未验。始终使用已有开发服务器，未访问生产站点。
