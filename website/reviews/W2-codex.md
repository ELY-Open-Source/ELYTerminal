FAIL

W2 第 1/3 轮审查完成。确认 1 个阻塞项：生产构建遗漏 Three.js 的 MIT 版权与许可文本。

范围：`website/package.json`、相关 lockfile 条目、`index.html`、`src/main.js`、`src/scene.js`、`src/style.css`、`public/ely.svg` 与三个字体及其许可证。同时核对 PRD 和 TASKS 中 W2 的脚手架、本地资产、clean install、production build 契约。审查日期：2026-10-28。

**F1 · 生产产物缺少 Three.js 许可证**

位置：`website/package.json:15`、`website/src/scene.js:1`，以及 `website/public/` 的发布资产集合。

`scene.js` 引入的 Three.js 0.186.1 已进入生产 JS。安装包的 `node_modules/three/LICENSE:12–13` 要求分发软件副本或实质部分时附带其版权声明和许可声明；其完整文本与 [Three.js 官方 MIT 许可证](https://github.com/mrdoob/three.js/blob/r186/LICENSE) 一致。

在全新临时副本执行 `npm ci` 和 `npm run build` 后，生成的 `dist/assets/index-CLm2Yhtb.js` 为 523426 bytes，包含 `WebGLRenderer` 实现；其中 `Copyright`、`SPDX-License-Identifier`、`@license` 和 `Permission is hereby granted` 均不存在。完整遍历 `dist/`（包含隐藏路径）得到 10 个文件：HTML、JS、CSS、Ely SVG、三个字体和三个字体许可证；Three.js 的许可证在产物中缺失。

构建 JS 的 SHA-256：`50216e15c562c2bb8259d85edf7ec6e5f1a918688b47e9465497c5c90b6ddcf2`。本次锁定版本的 Three.js LICENSE 为 1081 bytes；三个 OFL 文件仅涵盖相应字体。部署该静态产物会分发缺少所要求版权与许可文本的 Three.js 代码。

最小修正：把锁定版本 Three.js 的完整 LICENSE 作为 `website/public/three-LICENSE.txt` 随站点发布。复验时重建，确认以下比较成功，并确认该文件随静态产物可访问：

```sh
npm ci
npm run build
cmp node_modules/three/LICENSE dist/three-LICENSE.txt
```

**实际验证**

| Check | Evidence / result |
|---|---|
| Clean install | 在仓库外新建临时副本，排除原有 `node_modules`、`dist`，使用单独的 npm cache。Node 24.16.0、npm 11.13.0；`npm ci` 退出 0，安装 54 个包。package 与 lock 根依赖完全匹配，安装后的 lockfile 哈希保持一致。 |
| Production build | `npm run build` 退出 0；Vite 8.3.5 转换 8 个模块。JS 523.42 kB / gzip 130.28 kB，CSS 1.11 kB / gzip 0.56 kB，HTML 0.75 kB / gzip 0.41 kB。500 kB 提示按本轮约定归为尺寸警告。 |
| Local assets / font notices | `public/ely.svg` 与 `assets/icons/elyterminal.svg` 字节一致；全部 public 文件原样进入 dist。三个字体许可证各有对应作者版权与完整 OFL 1.1 文本，符合本轮检查的随附文本要求。[OFL 官方条款](https://openfontlicense.org/open-font-license-official-text/)。 |
| Browser startup / resize | Playwright 使用本机 Chrome 156.0.8078.48、headless、SwiftShader 加载生产 dist。1280×800、DPR 2 时画布为 1920×1200；改为 390×844 后画布为 585×1266，CSS 尺寸跟随视口。标题、Ely 图像及三个字体均成功加载；Space Grotesk 300/700、Zen Dots 400、JetBrains Mono 400/600 的 `document.fonts.load` 均通过。无未捕获页面异常、HTTP 错误或外部资源请求。 |
| WebGL failure path | 在脚本执行前令 WebGL context 创建返回 null，页面保留标题、Ely 图像和字体，`data-webgl=unavailable` 正确设置，应用记录 `ELYTerminal: WebGL initialization failed`。无未捕获异常，移动视口内容仍可读。 |

正常 WebGL 测试中的控制台提示来自 SwiftShader 的 `GPU stall due to ReadPixels`；强制初始化失败的测试出现 Three.js 与应用预期的错误日志。390×844 的截图已查看，用于核对脚手架内容可见。

Vite、原生 ES modules 与 Three.js 的职责保持在 W2 范围内：DOM 提供静态内容，固定全视口容器中的 Three.js 渲染背景，ResizeObserver 更新尺寸。最终布局、inline pose rig、滚动编排、完整 synthwave 场景、下载和语言/主题控制按 W3–W8 的任务归属留待实现。本轮发现仅为 F1。

**输入标识**

| Input | SHA-256 |
|---|---|
| `website/package.json` | `5dd772031c2de78b96219331f9f6688c83481d157cd099fedb2fe7cbfdc123c0` |
| `website/package-lock.json` | `bd84991d6a64937916f4308c5a6c03ff1f0eaf2b52b2a8061445d4ca70856296` |
| `website/src/main.js` | `9d7c6953910c73310fd4a5410f28201363cf4b74426d3a28d37a2475b0d4c507` |
| `website/src/scene.js` | `4e76efad5eead8e0fb915a08c00b5b9c464e073134377cd6e2b479ec0608aaaa` |
| `website/src/style.css` | `293089740376b7b4b7d16fc454472a3752b7fe2045c1bfc477f0f76409098844` |

所审手写文件均低于 500 行；package-lock 为生成文件。复验在临时副本执行，浏览器与本地服务器已关闭，临时副本随后清理；仓库写入范围为本报告。

未测试 Safari、Firefox、原生 GPU 性能或 Cloudflare 生产部署，后续任务的功能验收留待对应任务。当前发布风险为 F1 所述 Three.js 许可文本缺失。

