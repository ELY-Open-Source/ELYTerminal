PASS

W5 第 1/3 轮审查完成。当前范围可复现的阻塞缺陷为 0。审查日期：2026-10-28。

已读取本轮指定的十个文件，对照 PRD §2 和 TASKS 的 W5 条目，独立运行五项指定命令，查看全部十张 `target/website-shots/scene-*.png`，并执行 WebGL、系统外观、减弱动画和帧稳定性探针。W6 内容、W7 下载及 W8 手动控件/持久化按所属任务处理。

**指定命令**

为遵守写入范围，命令在仓库外的源码副本执行，依赖通过该副本的 `npm ci` 安装。构建和新截图均写入临时副本；浏览器脚本连接指定的 `http://127.0.0.1:5173`，已核对其 shader 源码与审查文件一致。

| Command | Result |
|---|---|
| `npm --prefix website test` | 退出 0；2 个测试通过。 |
| `npm --prefix website run build` | 退出 0；Vite 8.3.5，11 个模块，JS 532.11 kB / gzip 133.48 kB。保留已接受的 Three chunk 尺寸警告。 |
| `node website/tests/visual.mjs` | 退出 0；1440×900、1280×720、390×844、320×568、844×390、1280×600 全部通过，字体 loaded，errors / failures 均为空。 |
| `node website/tests/ely.mjs` | 退出 0；八张姿态捕获及减弱动画吸附通过。 |
| `node website/tests/scene.mjs` | 退出 0；两套配色、八张场景捕获、减弱动画静止比较及两套初始化失败回退通过。原脚本 90 个帧间隔平均为 16.67ms。 |

**职责、画面与系统外观**

Three.js 使用一个全屏平面及 fragment shader 绘制背景，canvas 位于固定的 `#scene` 中。独立浏览器测量确认背景边界为视口的 `[0,0,width,height]`，页面保持一个 canvas 和一个独立内联 SVG，Ely 由原 SVG rig 更新。

八张深浅场景图可见霓虹地平线、透视网格、条纹落日及扫描线/颗粒；产品名、场景标题、导航和终端画面可辨识。浅色使用独立的天空、地面、网格、落日和文字色值，Ely 保持原有粉色。两张提供的初始化失败截图可见 CSS 条纹落日和透视网格。

`main.js:7–12` 根据系统外观设置 data-theme，并监听系统变化。独立探针在桌面和手机仿真中执行 light → dark → light：CSS 文字色对应 `rgb(36,19,47)` / `rgb(255,248,253)`，shader 的 uTheme 同步为 1 / 0 / 1。减弱动画状态下切换系统外观也能更新画面。

**滚动、调度与减弱动画**

`main.js:14–24` 将 rig.update 和 scene.update 注册到同一个调度器。探针从页面启动前包装浏览器的原生 requestAnimationFrame，记录到的运行时回调仅为 `motion.js` 的 frame，最大待调度回调数为 1。背景跟随原生滚动，滚至 Git 后 uProgress 收敛至 2，Ely 同时进入 Git 姿态；Retina 桌面原生滚轮继续到达 scrollY=2700 的 Download 场景。

探针记录实际 WebGL uniform 和 draw 调用。切换 reduced motion 后，uTime 停止增加，350ms 内背景 draw 次数与 Ely transform 均保持相同。随后在 progress 0.49 / 0.51、1.49 / 1.51、2.49 / 2.51 两侧滚动，背景直接使用整数 0 / 1、1 / 2、2 / 3；发生重绘时，第一个 draw 已使用对应整数，Ely 也采用同一最近场景。浏览器动画数量为 0。回到普通动画模式后 uTime 恢复增加。

**WebGL 故障复验**

初始化失败：除原 scene.mjs 的桌面深浅测试外，独立探针在 320×568 手机仿真中让 WebGL getContext 返回 null。两套配色均记录 `ELYTerminal: WebGL initialization failed` 及具体创建失败原因；data-webgl 为 unavailable，canvas 数量为 0，CSS 回退可见。四个场景的 Ely 切换继续工作，页面宽度保持 320 / 320 / 320，page errors 为空。

运行中丢失 context：在正常创建并绘制的 WebGL2 context 上实际调用 `WEBGL_lose_context.loseContext()`。1440×900 桌面和 390×844 手机仿真均记录：
`ELYTerminal: WebGL context lost; using the static scene`。
canvas 随后移除，data-webgl 变为 unavailable，深浅 CSS 回退均可见。丢失后继续滚动至 Sessions，Ely 正常响应；切换系统外观也更新回退配色。减弱动画下间隔 200ms 的页面截图字节相同，实际 WebGL draw 计数保持停止；page errors 为空。

已目视查看上述探针生成的桌面、手机 context-loss 回退及 320px 初始化失败截图。故障模式下条纹落日、网格、Ely 和页面文字保持可见，未出现横向溢出。

**帧稳定性**

环境为 Node 24.16.0、Chrome 156.0.8078.48；WebGL 报告 `ANGLE Metal Renderer: Apple M4`。独立采样使用应用自身的 rAF 回调；回调耗时包括两个更新函数和同步渲染提交。

| Scenario | Drawing buffer | Samples | Mean / p95 / max interval | Mean / p95 callback time |
|---|---|---:|---|---|
| 1440×900，DPR 1，普通背景动画 | 1440×900 | 180 | 16.666 / 16.70 / 16.80ms | 0.456 / 1.00ms |
| 390×844，DPR 3 手机仿真，普通背景动画 | 487×1055 | 180 | 16.666 / 16.70 / 16.80ms | 0.448 / 0.90ms |
| 1440×900，DPR 2，原生滚轮穿过四场景 | 2160×1350 | 221 | 16.666 / 16.70 / 16.80ms | 0.402 / 0.60ms |

正常 WebGL 探针的 getError 为 0，console errors / page errors 均为空。两种独立视口及 Retina 滚动检查满足 `clientWidth === scrollWidth === innerWidth === configuredWidth`。本机采样未观察到帧间隔停顿；这些数据衡量本机调度间隔和同步回调耗时。

**源码约束与输入标识**

指定文件均低于 500 行，最大为 terminal.css 的 382 行；没有源码注释。未发现当前范围的死代码或版本化 API。场景结构锚点继续承担滚动进度定位职责。

| Input | Lines | SHA-256 |
|---|---:|---|
| `website/src/scene.js` | 155 | `f339ff4a3047c13adf7216d369574ad64fc9f80565a207c7594c7c712569deb7` |
| `website/src/main.js` | 29 | `6ceb4082f7562146f58f5fcdd7805fea7215d63f04f5725453d7a55368a55396` |
| `website/src/motion.js` | 18 | `ae7d92b21b17df785a4ba49d3fc5255b8ec3e9068901558f5388057a02dfd0f1` |
| `website/src/ely.js` | 149 | `dc3d7a096b85779430d28d21e2ee02212cb35572db288fceb65310a9347f2ce7` |
| `website/src/style.css` | 360 | `f526b440339708087a65af409ca0a688fd1b957417d78e7973f4118a84dac83a` |
| `website/src/terminal.css` | 382 | `d2226c324058a19e65dd2b5744b445b62c3e71ce927abac6da7944a9341d4e4c` |
| `website/index.html` | 179 | `6d709b9df7d7b50b4227c50bfdde93e015bd0badb542e00fe8a32465736d4423` |
| `website/tests/scene.mjs` | 83 | `c1e191e063c7f74731806553d7efa503f0786b943950564b39cb10eb57f255e2` |
| `website/tests/ely.mjs` | 73 | `e14bdf313cdb6d7af55136313b0fa9f32d494ff8a4ffe9a24b119bd7f163e619` |
| `website/tests/visual.mjs` | 82 | `a57a6287a54e9374d0c7b08704496fe324f8da9e1207229f88cbde637b2f4556` |

独立浏览器已关闭，仓库外临时副本已清理。本轮唯一主动写入的仓库文件为 `website/reviews/W5-codex.md`；指定十个审查文件及 `target/website-shots` 的 29 张原截图均与起始 SHA-256 一致。全树指纹发生变化，修改时间检查显示现有开发服务器的 `website/node_modules/.vite/deps/` 缓存发生重写，涉及 `_metadata.json`、`three.js`、`three.js.map` 和该目录的 `package.json`；这是本轮检查期间记录到的额外文件系统副作用。指定开发服务器保持运行。

未测试 Safari、Firefox、实体手机 GPU 或生产部署；手机和高 DPR 结论来自本机 Chrome 仿真，context loss 来自真实 WebGL 扩展触发。当前 W5 范围未发现阻塞风险。

