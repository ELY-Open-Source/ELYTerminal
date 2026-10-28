PASS

W8 第 1/3 轮审查，2026-10-28。依据本轮所有者指定的 Aurora / Glassmorphism 外观、最新 PRD.md 和 TASKS.md 审查。代码、实际渲染、独立像素对照及回归检查通过，未发现可复现且阻碍 W8 验收的问题。W9 的语言/主题按钮及持久化保持在后续任务范围内。

**视觉与玻璃效果**

已逐张查看最新深浅两色的 Terminal、Sessions、Git / Files、Download 桌面截图，移动端 hero、Sessions、Git、Files、Download，以及深浅 WebGL fallback。另查看本轮独立生成的移动端深浅场景和 context lost 截图。背景呈柔焦的青、蓝、薄荷及暖桃色光场；窗口、导航及主下载操作使用半透明表面、细边缘高光和柔和彩色阴影。标题、窗口内容及下载文字可读，应用内部功能窗格保持既有结构。

源码和截图均已移除旧透视网格、条纹落日、扫描线、VHS 外观、复古标题字体及描边标题。目视未见紫色渐变。Ely 内联 SVG 与 public/ely.svg 均与 W7 通过版本逐字节相同，原有粉色原画及眼睛固有紫色得到保留。HTML 差异只有 theme-color 和旧字体 preload 的移除；没有新增营销嵌套卡片、标签、徽章、tagline 或 filler。Zen Dots 字体文件、许可证、加载声明和引用均已删除。

玻璃验证使用实际页面中的 app-window 和 terminal-screen，在独立浏览器内暂时替换其后方背景，保持窗口本身的颜色、叠层及尺寸。1440×900 下采样终端空白区域 x=650、y=800、300×40px，分别捕获黑白条纹在原有模糊、临时关闭 backdrop-filter，以及纯黑/纯白背景下的结果。所有诊断修改只存在于测试浏览器页面中。

| 主题 | 条纹经原有模糊后的 RGB 标准差 | 关闭模糊后的 RGB 标准差 | 背景黑→白引起的窗口内 RGB 变化 |
|---|---|---|---|
| 浅色 | 0 / 0 / 0 | 52 / 52 / 52 | 每通道 +104 |
| 深色 | 0 / 0 / 0 | 31 / 31 / 31 | 每通道 +62 |

此对照确认实际渲染透过底层光线，同时将底层细节模糊。原页面也可见 Ely 位于窗口后方的部分形成柔化色影。计算样式为 blur(32px) saturate(1.45)，浅色窗口底色 alpha=0.42、深色 alpha=0.58；阴影分别使用低透明度的蓝灰色和青蓝色。

**独立行为与布局检查**

两种系统外观分别覆盖 1440×900、1280×720、390×844、320×568、844×390、1280×600，共 48 个场景状态。逐场景检查 scrollWidth、clientWidth、innerWidth 与配置宽度一致，标题和应用/下载区域位于水平边界内，标题与应用分离。下载检查实际滚至页面底部后的操作区可见性，以及 Ely 与标题、操作区分离。导航链接边界、Space Grotesk 标题字体、零描边及移动端 Files 页签点击均通过，page errors 为空。

独立隔离前景后的背景截图对照确认：普通模式下一秒后光场像素缓慢变化；减弱动画模式下一秒前后像素完全一致；两种模式下滚动均改变场景背景，系统外观切换均更新实际背景像素。减弱动画与四场景姿态的既有回归也通过。

额外使用 WEBGL_lose_context 在 390×844 的深浅主题分别触发真实 context lost。两次均记录 `ELYTerminal: WebGL context lost; using the static scene`，移除 canvas，并显示可读的静态 Aurora 回退，页面宽度保持 390px。初始化失败路径由 scene.mjs 独立执行并捕获深浅截图。

**独立执行结果**

所有安装、构建、测试和新截图均在 `target/website-review-w8/run-05h_mtr_/` 的源码副本与临时目录执行；浏览器连接原开发服务器。以下命令全部退出 0。

| Command | Result |
|---|---|
| `npm --prefix website ci --no-audit --no-fund` | 干净安装通过。 |
| `npm --prefix website test` | 6/6 通过，包括平台、别名及姿态单元检查。 |
| `npm --prefix website run build` | 14 个模块；JS 533.06 kB / gzip 133.99 kB，仅保留已接受的 Three chunk 警告。 |
| `node website/tests/visual.mjs` | 六视口通过；字体 loaded，errors / failures 为空。 |
| `node website/tests/ely.mjs` | 八张姿态捕获与减弱动画吸附通过。 |
| `node website/tests/content.mjs` | 六视口、双语数据、鼠标/键盘页签及窗格末尾可达性通过。 |
| `node website/tests/scene.mjs` | 深浅主题、八场景截图、减弱动画像素稳定和 WebGL fallback 通过；平均帧间隔 16.67ms。 |
| `node website/tests/downloads.mjs` | 五组平台输入、六链接、六视口及 W7 同页缩窗回归通过。 |

自编探针为该临时目录内的 `website/aurora-review.mjs`，数据为 `aurora-review.json`、`glass-pixels.json`、`motion-pixels.json`。`glass-*.png` 保存透光/模糊对照，`mobile-*.png` 保存新增移动端截图，`commands.log` 和 `commands.json` 保存完整回归结果；原测试的新截图位于该目录的 `target/website-shots/`。

**源码约束与边界**

运行时改动集中于 scene.js 和三份 CSS；Ely rig、单一 rAF 调度器、main.js、页签模块、OS 下载映射与 release-asset.sh 均与 W7 通过版本保持相同。Three.js 继续只拥有固定背景。重点手写文件为 scene.js 129 行、style.css 364 行、terminal.css 404 行、scenes.css 333 行、index.html 276 行，均符合 500 行上限；改动源码没有新增注释。

| Input | SHA-256 |
|---|---|
| `website/src/scene.js` | `89dfbfc508460a35d96fc7e83fdc0545b1e27e66208ddb39acfd8e9f15d82b97` |
| `website/src/style.css` | `0e873f185fd762318ec144bfd3262b75ebd2de65ac0e39d143fcd00e328859e6` |
| `website/src/terminal.css` | `5a18bac40915b67f57207f1aab7419f093dbc63aca9766f0e84cf8d96b8e8743` |
| `website/src/scenes.css` | `573ccd37748a7db340c005c450634fddcd3897b861b0ec7df671b1907dbdf0f4` |

除指定临时目录外，本轮仅新增本报告。起始文件及 41 张原截图的 SHA-256 已复核；实现、公开 release、远端资源和开发服务器保持原样，独立浏览器已关闭。

浏览器证据来自 Chrome 156.0.8078.48。实体移动设备、原生 Safari/Firefox、公开下载和生产部署未验证；本 PASS 限于 W8 外观替换及本轮实际执行的回归范围。
