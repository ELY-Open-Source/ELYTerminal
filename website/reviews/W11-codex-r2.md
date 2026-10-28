FAIL

W11 第 2/3 轮。对照第一轮报告核验当前修复：页面就绪后的双向切换、开场中途切换，以及两项静态规则问题均已关闭。F1 在异步初始化期间仍有一条可复现的遗漏路径。

**F1：WASM 加载期间发生的减弱动画变化被丢失。**

位置：`website/src/stage.js:68`、`:75`、`:78`、`:85`、`:266`、`:288`。`reduced` 在请求 WASM 前读取；中间等待 WASM 与字形初始化，随后才注册 `change` 监听。等待期间发生的变化已经结束，注册监听时也没有重新读取当前偏好，因此初始化分支、乱码开关和绘制循环继续使用旧值。

复现使用现有开发服务器，仅在浏览器路由中暂缓本地 `swarm.wasm` 响应，稳定触发正常的异步加载窗口；页面源码、媒体查询 API 和服务器均保持原样。

1. 以 `reducedMotion: 'no-preference'` 打开页面，等待浏览器发出 WASM 请求并暂缓该响应。
2. 同页调用 `page.emulateMedia({ reducedMotion: 'reduce' })`，等待 150ms，再放行请求。
3. 等待加载、开场和成形完成，间隔 500ms 采样画布、实际 WASM 输出及 WebGL uniform。

实测媒体查询已为 reduce，`uScramble` 仍为 1；draw 次数从 277 增至 317，粒子位置继续改变，两张截图不同。日志仍出现 `intro spells 517 cells` 与 `stage ready, full motion`。反向复现也成立：初始 reduce，在等待响应时改回 no-preference，加载后 `uScramble=0`、draw 次数保持 2、粒子位置和截图保持静止。

影响：当前系统偏好与字形群长期不一致；在加载期间启用减弱动画的访客仍会看到开场、漂移及乱码。该路径属于原 F1 的状态同步问题。

最小修复：异步初始化完成后，使用当时的 `motion.matches` 统一初始化 `reduced`、`uScramble` 和开场/静态分支，保留现有实时监听。回归加入“暂缓 WASM 响应 → 改变偏好 → 放行响应”的两个方向，并核验实际运动状态。

证据：[probe.json](../../target/website-review-w11-r2/run-ecqs2_0x/probe.json) 的 `loading`、[probe.log](../../target/website-review-w11-r2/run-ecqs2_0x/probe.log)、[独立探针](../../target/website-review-w11-r2/run-ecqs2_0x/probes/probe.mjs)。

**已关闭的原问题。**

| 路径 | 独立实测 |
| --- | --- |
| 成形后 full → reduce | `uScramble=0`，粒子位置与 draw 次数固定，画布像素稳定；按压后高速粒子数为 0 |
| 初始 reduce → full | `uScramble=1`，draw、粒子位置和像素恢复变化；按压产生高速粒子 |
| 开场中途启用 reduce | 结束开场，4777 个有效 Ely 单元成形；待标题颜色过渡结束后，像素与物理状态持续稳定 |
| 静态规则 | 未用 RAMP 导入已删除；`ely.js:91` 注释为十词；旧 `places()` 已删除，检查的相关手写文件最大 485 行，注释均为一行且不超过十词 |

**验证范围与结果。**

在 target 下的临时副本独立执行干净安装、`npm --prefix website test`（33/33）、`npm --prefix website run build`，以及 `preferences.mjs`、`downloads.mjs`、`content.mjs`、`stage.mjs`、`visual.mjs` 五套浏览器测试，全部通过。构建保留已接受的 Three chunk 大小警告；stage 平均帧间隔 16.67ms。浏览器始终连接已有 `http://127.0.0.1:5173`。

重新编译、工作区及服务器返回的 WASM 哈希一致，见 [wasm-hashes.json](../../target/website-review-w11-r2/run-ecqs2_0x/wasm-hashes.json)。独立探针读取真实 WASM 粒子状态和 WebGL 的 `uScramble`，并比较截图；像素比较等待既有标题颜色过渡完成。

原生滚动、四场景成形、双主题和双语言回归已检查。WASM 404、WebGL 初始化失败及真实 context lost 都显示静态 Ely，并记录明确错误；正常路径未发现 pageerror 或 console.error。页面阅读过程中的位置插值问题归入 [W12 本轮报告](W12-codex-r2.md)。

完整证据见 [本轮目录](../../target/website-review-w11-r2/run-ecqs2_0x/)。临时构建目录、安装依赖与缓存已删除；既有 746 个文件、87 张截图及原 WASM 哈希保持一致。仓库仅新增本轮两份报告。

验证边界：本机 Chrome 156.0.8078.48 与视口/媒体模拟；未运行 Safari、Firefox、实体触屏设备或实际读屏软件。未访问生产站点。
