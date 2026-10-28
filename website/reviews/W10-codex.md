PASS

W10 最终本地验收，2026-10-28。依据 website/PRD.md §7、当前全站要求及 TASKS.md W10，独立检查通过，未发现需求、正确性、可访问性、控制台或视觉阻塞。W9 两项修复继续通过。

本轮读取 PRD、TASKS、W9-r2 报告、页面、样式、全部运行模块和六个浏览器套件。与 W9-r2 审查快照相比，源码仅 TASKS.md 的完成状态变化。全部临时文件、源码副本、安装、构建、日志和截图位于 `target/website-review-w10/run-dg11aj39/`。浏览器使用原有 `http://127.0.0.1:5173`。

独立布局与交互矩阵覆盖英文/简体中文 × 深色/浅色 × 六视口，共 24 组：1440×900、1280×720、390×844、320×568、844×390、1280×600。390px、320px 的场景矩阵启用移动视口和触摸能力。每组均遍历 Terminal、Sessions、Git、Files、Download，核对页面宽度、标题与应用边界、页签状态、窗格末尾及下载操作区；全部通过。正常场景、滚动和键盘路径的 page errors、console warning/error 及失败请求记录均为空。

已目视检查本轮重新生成的八张 `scene-{dark,light}-{terminal,sessions,git,download}.png`、三张 `content-390-{sessions,git,files}.png`、`download-844x390-bottom.png`；另检查独立捕获的英文深色桌面四场景、英文浅色手机四场景，以及深浅静态回退。标题、正文、下载链接和页签可读，短屏下载标题及操作区与 Ely 分离。Aurora、透光玻璃与原有粉色 Ely 保持一致；页面内容保持连续产品场景，未出现营销嵌套卡片或新增填充文案。

原生滚动与姿态通过。桌面真实滚轮使 scrollY 从 0 到 300；移动上下文使用浏览器触摸滚动手势使 scrollY 从 0 到 301。Ely 的位置、缩放、头部、手掌和视线分别记录到初始、中间和稳定状态，确认连续插值。既有 Ely 套件验证桌面/手机四个不同姿态、表情和下载闭眼状态。短屏自然滚动、1280×720 同页缩至 1280×600，以及页面底部的下载布局继续通过。

帧与减弱动画通过。排除测试工具自身等待帧后，页面 rAF 待执行数及最大数均为 1；既有帧稳定性检查平均间隔为 16.67ms。运行中切换 prefers-reduced-motion 后，Ely 直接吸附到最近场景，背景截图连续两次完全相同，等待期间 WebGL 绘制计数保持不变：桌面 106→106，手机 136→136。继续滚动至下一场景附近时吸附到新姿态，随后保持稳定。

首次帧计数探针使用 Playwright 默认 rAF 轮询，计入了测试工具自身的等待帧并得到 2。改为定时轮询后独立重跑运动和回退部分通过；页面实现保持原样。首轮矩阵及键盘结果保存在 `acceptance-matrix.json`，首轮探针日志保存在 `acceptance-first.log`，合并结果保存在 `acceptance-review.json`。

键盘与内部窗格通过。Tab 可到达跳过链接、导航及两个偏好控件，Enter 可跳转场景；语言和主题按钮支持 Enter/Space，名称随语言与目标主题更新。Git/Files 的鼠标、左右方向键、Home/End、焦点、aria-selected 和面板显隐一致。另实际用 Tab 进入溢出窗格、End 滚至末尾，并用滚轮分别核对相同目标。

| 视口 | Sessions 最后一行 / idle | Git 最近提交 | Files PRD.md |
|---|---|---|---|
| 320×568 | 可达，scrollTop 49px | 可达，scrollTop 26px | 可达，scrollTop 84px |
| 1280×720 | 可达，scrollTop 3px | 全部可见 | 可达，scrollTop 39px |

两尺寸均可依次 Tab 到达六个下载格式链接，焦点所在链接完整处于视口内。独立偏好探针再次覆盖 24 组导航布局：两控件均为 44×44px，header 内容完整容纳；320px 英文主题按钮焦点环右边为 308.09375px。中文 Terminal 导航文本及可访问名称为“终端”，英文为 Terminal。

偏好与异常路径通过。en-US 默认英文，zh-CN、zh-TW、zh-Hans-CN 默认简体中文。两方向显式语言选择和主题选择在刷新后保留；未手动选择时跟随系统，手动主题选择后保存值优先于后续系统变化。localStorage 属性访问、getItem、setItem 分别抛错及保存值无效时，页面和控件继续工作，受阻操作有明确警告。保存失败时，本页手动主题仍优先于系统变化。

本轮构建产物的八组首次绘制检查通过：系统深浅、相反保存选择、无效保存值、存储拒绝。浏览器请求拦截保持主模块等待时，first-paint/first-contentful-paint 已使用正确主题和背景；放行主模块后主题保持正确。

WebGL 回退通过。深浅两色分别强制初始化失败与实际 WEBGL_lose_context，共四组。canvas 移除后显示对应的静态渐变背景；语言、主题、滚动和 Files 页签继续工作，page errors 为空。初始化失败产生 Three 诊断及 `ELYTerminal: WebGL initialization failed`；上下文丢失产生 `ELYTerminal: WebGL context lost; using the static scene`。这些日志均对应本轮主动注入的故障。

下载检测与公开目标通过。浏览器测试覆盖普通 macOS、Windows、Linux、Android、桌面模式 iPad；主链接分别选择 DMG、ZIP、AppImage 或 releases 页面。六个格式 href 与规定的版本无关别名一致；中文平台标签也独立核对。于 2026-10-28 15:23:33 UTC 使用 `curl -I -L` 独立验证下列目标，公共前缀为 `https://github.com/ELY-Open-Source/ELYTerminal/releases/latest/download/`。

| Asset | 最终状态 | 重定向次数 |
|---|---:|---:|
| ELYTerminal-macos-arm64.dmg | 200 | 2 |
| ELYTerminal-windows-x86_64.zip | 200 | 2 |
| ELYTerminal-linux-x86_64.AppImage | 200 | 2 |
| elyterminal_amd64.deb | 200 | 2 |
| elyterminal.x86_64.rpm | 200 | 2 |
| ELYTerminal-linux-x86_64.tar.gz | 200 | 2 |

独立运行的既有检查全部退出 0：

| Command | Result |
|---|---|
| `npm --prefix website ci --no-audit --no-fund` | 隔离副本干净安装通过。 |
| `npm --prefix website test` | 7/7 通过，包含下载、姿态、偏好和发布别名脚本测试。 |
| `npm --prefix website run build` | 15 个模块；JS 534.56 kB / gzip 134.57 kB；保留已接受的 Three chunk 警告。 |
| `node website/tests/visual.mjs` | 六视口、字体加载、页面宽度、标题和短屏滚动通过。 |
| `node website/tests/ely.mjs` | 八张姿态捕获及减弱动画吸附通过。 |
| `node website/tests/content.mjs` | 六视口、双语内容、页签和内部窗格可达性通过。 |
| `node website/tests/scene.mjs` | 两色四场景、帧间隔、减弱动画和静态回退通过。 |
| `node website/tests/downloads.mjs` | 五平台、六目标、六布局和同页缩窗回归通过。 |
| `node website/tests/preferences.mjs` | 语言、主题、持久化、系统变化、键盘、存储和六布局通过。 |

证据索引：`commands.log`、`commands.json` 记录既有检查；`acceptance-review.json`、`pane-keyboard-review.json` 记录场景矩阵、滚动、键盘、帧和回退；`preferences-review.json`、`prepaint-review.json` 记录偏好及首次绘制；`release-heads.json` 和 `heads/` 保存六个 HEAD 结果及响应头。独立脚本位于临时副本的 `website/`，截图位于 `acceptance-shots/`、`target/website-shots/` 及临时目录根部。

源码检查确认单个内联 SVG、统一帧调度器、原生滚动和现有模块划分继续保持；运行代码未引入版本化 API 或死代码。重点手写文件均不超过 500 行，最长为 style.css 的 448 行；审查脚本均不超过 500 行，未添加注释。

起始 4076 个文件和 44 张原截图已按 SHA-256 核对，输入哈希清单位于 `baseline.json`。指定临时目录外仅新增本报告；实现、开发服务器、公开 release 和远端资源保持原样，独立浏览器均已关闭。

浏览器证据来自 Chrome 156.0.8078.48，手机为浏览器模拟视口及触摸输入。原生 Safari/Firefox、实体移动设备和实际读屏软件未验证。部署与生产验收属于 W11，本轮未执行。
