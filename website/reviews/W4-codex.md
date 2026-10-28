PASS

W4 第 1/3 轮审查完成。当前范围可复现的阻塞缺陷为 0。审查日期：2026-10-28。

范围为本轮指定的九个文件。已对照 PRD §4、TASKS 的 W4 条目和本轮约束，读取实现及测试，独立运行指定命令，查看八张 `target/website-shots/pose-*.png`，并执行独立浏览器探针。

**指定检查**

为保持仓库文件原状，命令在仓库外的源码副本执行；生产构建和新截图均写入该副本。两个浏览器脚本使用指定的 `http://127.0.0.1:5173` 开发服务器，已核对该服务器提供的 Ely 模块与本次源码一致。Node 为 24.16.0，Chrome 为 156.0.8078.48。

| Command | Result |
|---|---|
| `npm --prefix website test` | 退出 0；2 个测试通过。 |
| `npm --prefix website run build` | 退出 0；Vite 8.3.5，11 个模块。JS 527.17 kB / gzip 131.76 kB。已接受的 Three chunk 尺寸警告继续保留。 |
| `node website/tests/visual.mjs` | 退出 0；六组全部通过，fonts loaded，errors / failures 均为空。 |
| `node website/tests/ely.mjs` | 退出 0；桌面及手机共八张场景捕获、四种表情和 reduced-motion 吸附检查通过。 |

临时依赖副本最初展开了 `.bin/vite` 符号链接，导致隔离构建路径错误；在临时副本执行 `npm ci` 恢复依赖后，生产构建通过。该问题发生在审查副本的准备阶段。

**姿态与 SVG**

`index.html` 保留一个内联 SVG，增加表情路径并复用 Ely 的原有分层图形。`src/ely.js:108–128` 更新已有元素的 transform、opacity 和 display；源码中未发现 innerHTML 重建或原生栅格 rig 移植。

八张桌面/手机姿态截图均已目视检查。四场景的位置、头部和表情可区分，Ely 保持可见。独立 DOM 测量得到以下场景端点；头部角度单位为度，视线及手掌位移为 SVG 坐标单位：

| Scene | Head | Gaze x / y | Left / right hand y | Expression |
|---|---:|---|---|---|
| Terminal | 0 | 0 / 0 | 0 / 0 | smile，睁眼 |
| Sessions | -4 | 5 / 5 | -5 / 1 | flat，睁眼 |
| Git | 6 | -4 / 2 | 1 / -3 | gasp，睁眼，感叹号 |
| Download | -3 | 0 / 0 | -5 / -5 | grin，弧形眼，星光 |

六个视口分别采样 progress 0 到 3、步长 0.25，共 78 个状态。每段中点的位置、头部、视线、双手属性均区别于相邻两个端点，验证了场景之间的数值插值。原生滚轮后，首个普通动画帧产生中间姿态，随后收敛到目标姿态。

**调度、滚动与减弱动画**

独立探针在页面启动前包装 requestAnimationFrame，并按 60Hz 时间步推进原回调。每个视口均推进 3249 帧；记录到的运行时回调仅为 `motion.js` 的 `frame`，最大待调度回调数为 1。源代码中启动和续帧均属于这个调度链，`main.js` 只创建一次调度器并注册 rig.update。

探针保存 SVG 及全部后代节点引用，并观察 childList。六组均为：SVG 数量 1、节点引用保持相同、childList 变更 0。四场景与中间状态保持同一套 DOM 节点。

六组均通过真实浏览器滚轮及 PageDown：滚轮使 scrollY 从 0 到 350，PageDown 继续增加 scrollY；相关 wheel、touchmove、keydown、scroll 事件的 preventDefault 调用记录为空。源码亦未注册滚动拦截器。

在页面运行中切换 `prefers-reduced-motion: reduce`，分别检查 progress 0、0.49、0.51、1.49、1.51、2.49、2.51、3。六组共 48 次检查均在一个调度帧内精确采用最近场景的位移、头部、视线、双手和表情。每次继续推进 180 帧后姿态保持相同，运行中的浏览器动画数量为 0；CSS smooth scrolling 同时关闭。初始 reduced-motion 页面也通过原 `ely.mjs` 检查。

**响应式与源码约束**

| Viewport | Scene anchors | Result |
|---|---|---|
| 1440×900 | 0 / 900 / 1800 / 2700 | 13 个采样通过 |
| 1280×720 | 0 / 720 / 1440 / 2160 | 13 个采样通过 |
| 390×844 | 0 / 844 / 1688 / 2532 | 13 个采样通过 |
| 320×568 | 0 / 704 / 1272 / 1840 | 13 个采样通过 |
| 844×390 | 0 / 704 / 1094 / 1484 | 13 个采样通过 |
| 1280×600 | 0 / 704 / 1304 / 1904 | 13 个采样通过 |

全部采样满足 `clientWidth === scrollWidth === innerWidth === configuredWidth`，Ely 容器与变换后的 SVG 图形保持横向边界。手机组启用了 isMobile / hasTouch。另以实际浏览器调度检查 1440×900 → 390×844 → 844×390 → 320×568 → 1440×900 的动态调整，桌面及手机仿真上下文均保持宽度正确，Ely 按新视口重新定位。

本轮源码审查未发现死代码或版本化 API。指定九个手写文件均少于 500 行，最大为 terminal.css 的 382 行；这些文件没有源码注释，满足注释约束。

W5 背景、W6 场景内容、W7 下载与 W8 控件按所属任务处理。Sessions / Git / Download 的结构锚点承担 W4 的滚动定位职责。

**输入标识**

| Input | Lines | SHA-256 |
|---|---:|---|
| `website/index.html` | 179 | `6d709b9df7d7b50b4227c50bfdde93e015bd0badb542e00fe8a32465736d4423` |
| `website/src/ely.js` | 149 | `dc3d7a096b85779430d28d21e2ee02212cb35572db288fceb65310a9347f2ce7` |
| `website/src/motion.js` | 18 | `ae7d92b21b17df785a4ba49d3fc5255b8ec3e9068901558f5388057a02dfd0f1` |
| `website/src/main.js` | 21 | `72ef4bbdf22f8174b1aedf47a2726eccbc37d56212f058a485b28083a0b350c8` |
| `website/src/style.css` | 297 | `675cf1ed616f0438be38187c1cb04ea81fc701a4065acd2c3505deb843ae648f` |
| `website/src/terminal.css` | 382 | `d2226c324058a19e65dd2b5744b445b62c3e71ce927abac6da7944a9341d4e4c` |
| `website/tests/ely.test.js` | 34 | `eb36557d2974be78b7dc5d2baa4fc33d1057a8f9064a1297b49eb58ca2a3725f` |
| `website/tests/ely.mjs` | 73 | `e14bdf313cdb6d7af55136313b0fa9f32d494ff8a4ffe9a24b119bd7f163e619` |
| `website/tests/visual.mjs` | 82 | `a57a6287a54e9374d0c7b08704496fe324f8da9e1207229f88cbde637b2f4556` |

浏览器探针已关闭，仓库外临时副本已清理。文件哈希核对确认本轮仅写入 `website/reviews/W4-codex.md`；原有源码及 `target/website-shots` 的 17 张截图保持原状，构建产物写入临时副本，指定开发服务器保持运行。

未测试 Safari、Firefox、实体手机触摸手势或生产部署；移动端结论来自 Chrome 仿真。当前 W4 范围未发现阻塞风险。

