FAIL

W15 第 1/3 轮。发现 1 项文档准确性阻塞 F1；仓库指定 gate、网站单元测试、构建及六套本地浏览器检查通过。审查对象为当前工作区，包含未提交修改。

F1：AGENTS.md 的开发服务器地址与所列命令实际监听地址不符。

位置：`AGENTS.md:35`；相关实现为 `website/package.json:8`，浏览器套件默认地址见 `website/tests/preferences.mjs:7`。

根因：文档声称 `npm --prefix website run dev` 会提供 `http://127.0.0.1:5173`。脚本实际执行 `npm run wasm && vite`，没有指定 host，也没有 Vite 配置覆盖默认值。当前环境的 localhost 优先解析为 IPv6 `::1`，Vite 8.3.5 因而仅监听 IPv6 loopback；浏览器套件默认访问 IPv4 `127.0.0.1`。

独立复现：在工作区副本中保留原脚本，只改用空闲端口并指定 strictPort，避开所有者的 5173：

```sh
npm --prefix website run dev -- --port 63226 --strictPort
curl --noproxy '*' -I http://localhost:63226/
curl --noproxy '*' -I http://127.0.0.1:63226/
```

实际使用 GET 核验：localhost 与 `[::1]` 返回 200；127.0.0.1 返回 curl 退出码 7、HTTP 000、connection refused。`lsof` 显示唯一监听地址为 `[::1]:63226`，Vite 日志打印 `http://localhost:63226/`。

影响：按文档启动后，指定的 IPv4 地址不可访问；默认 e2e 也无法连接该服务。W15 明确要求每项命令说明与代码一致。

最小修复：将 AGENTS.md 中的启动示例写为 `npm --prefix website run dev -- --host 127.0.0.1`，使文档地址与现有测试默认地址一致。本轮已用同一脚本加该 host 参数、在隔离端口 64277 复验，127.0.0.1 返回 200；源码保持原样。

证据目录：`target/website-review-w15/run-q7flks00/`。F1 证据为 `dev-server.log`、`dev-listener.txt`、`dev-host-probe.json`；最小修复验证为 `dev-host-flag.log`、`dev-host-flag-listener.txt`、`dev-host-flag.json`。

AGENTS.md 其余主张核对：

| 位置 | 核验结果 |
| --- | --- |
| :15 Stack | package.json 固定 Vite 8.3.5、Three.js 0.186.1、anime.js 4.5.0、Wrangler 4.154.0；swarm 为无外部依赖的 Rust crate，生成 WASM；wrangler.jsonc 配置静态 assets 与 terminal.elygpui.com Custom Domain。说明一致。 |
| :35 Commands | WASM 先构建再 dev；test 构建 WASM 后执行单元测试；test:e2e 串行执行六个浏览器脚本，截图写入仓库 target/website-shots；deploy 为 build 后 wrangler deploy。除 F1 地址外，与脚本一致。 |
| :52 Layout | main 动态加载 stage，motion 调度 stage.update；stage 读取当前场景 slot 的实时矩形，ely 负责姿态及格子尺寸，grid 采样，glyphs 使用单个实例化 mesh，physics 对接 WASM；type/orbs、preferences、downloads、scenes 的职责与描述一致。 |
| :155 下载别名决策 | downloads.js 使用六个无版本别名；release-asset.sh 为受支持的版本化包追加别名，--alias-only 只上传别名。单元测试通过 mock cargo/gh 验证上传次数、名称和未知别名失败。latest 排除草稿与预发布的说明符合 [GitHub 官方文档](https://docs.github.com/en/rest/releases/releases#get-the-latest-release)。 |

仓库 gate：

在副本中按原工作区 `git ls-files -co --exclude-standard` 的 755 个文件建立隔离索引，提取 `scripts/check.sh:7–17`，保留 `set -eu`，以 `sh` 执行；退出 0。执行范围为现有 Rust 连续行注释规则与文本文件行数规则，原生应用的 fmt/clippy/cargo test 未运行。

旧行数逻辑在同一副本退出 1，唯一输出为 `website/package-lock.json has 2459 lines`。新逻辑按文件名豁免 Cargo.lock 和 package-lock.json，与 `AGENTS.md:56` 一致。独立隔离用例确认：根目录和子目录的两类 501 行锁文件通过；500 行普通源码通过；501 行普通源码失败；相邻两条 Rust 行注释失败。证据：`house-gate.sh`、`gate.log`、`gate-result.json`、`old-gate.log`、`gate-fixtures.json`。

网站自有手写文件的注释扫描共检查 26 条实际注释，全部为单行、最多十词；最大手写文件为 style.css 的 490 行。scripts/check.sh 的说明注释也是单行、七词。词数核验范围为网站与本次脚本；仓库 Rust 按现有 gate 的连续行规则执行。证据：`comments.json`。

最终 PRD 对照：

| PRD 范围 | 当前构建与验收证据 |
| --- | --- |
| §1、§3 页面与产品叙事 | Terminal、五个 Sessions 状态、Git/Files 页签及 diff stat、系统下载和六种格式均存在；英文/简体中文及两主题通过，页面保持平面等宽布局，未见新增营销嵌套卡片、tagline 或填充描述。 |
| §2、§4 字形与动画 | SVG 姿态经网格转为字形，Rust WASM 处理粒子，Three.js 单个实例化 mesh 绘制；开场、重组、乱码、指针、按压、reduce 初始与实时/加载期间双向切换均通过。真实 context lost、WASM 失败、WebGL 初始化失败均有明确日志及可读静态回退。 |
| §5 下载 | 平台映射、桌面模式 iPad、六个稳定地址及发布脚本别名测试通过；链接保持 latest/download。公开下载 HTTP 结果沿用 W14-r2，本轮产物与其一致。 |
| §6 实现及规则 | Vite、vanilla JS、Three.js、anime.js、Rust WASM 与本地资源符合实现；所读网站模块和依赖中未见新增框架、后端、数据库、账号、分析或远端运行时 API。关键失败路径明确报错，本次改动未引入运行时代码或死代码。 |
| §7、§8 验证及部署 | 当前副本的干净 npm ci、32/32 单元测试、build、preferences/downloads/content/placement/stage/visual 全部通过。仅已接受的 Three chunk 警告；帧间隔均值 16.67ms。9 个构建文件的 SHA-256 全部与 W14-r2 已核验线上产物一致，部署配置与回滚记录保持一致。 |

§4 的落地细化沿用 W12-r3 已接受的方案：人物跟随当前场景的预留 slot，场景切换使用 0.58 迟滞，粒子飞行完成过渡。该方案用于保持滚动阅读时人物与正文分离；本轮逐段滚动、同页缩放及文字/槽位检查均通过。

W11 仍按 `website/TASKS.md:15` 保留未勾选、三轮封顶、交由所有者处理的状态。本轮记录其现状：最后一轮的失效画布问题已有隐藏 stage 的修复，W13 已验收，本轮 stage 套件再次通过真实 context lost 检查；W15 不替所有者关闭 W11，也不新增 W11 审查轮次。

本地浏览器仅连接隔离生产预览 `http://127.0.0.1:63226`。六视口、两语言、两主题的布局/控制检查，以及五尺寸 × 两主题 × 四场景截图通过；已目视核验八张桌面场景及手机、320px、短横屏样本。另用原生滚轮、DOM 命中及 CDP 仅视口截图核验固定页眉。证据：`browser.log`、`shots/`、`header-probe.json`、`header-cdp-false.png`。

源码与构建证据：`source.tar.gz` 保存本轮工作区快照；`runtime-comparison.json` 表明除任务/审查文档外，网站文件与 W14-r2 一致；`unit.log`、`build.log`、`build-comparison.json` 保存本轮执行与产物对照。所有自建服务器已停止，隔离构建及依赖已删除；原有文件哈希保持不变，仓库只新增本报告，见 `integrity.json`。

验证边界：本轮未执行原生应用 cargo test、生产浏览器/Playwright、部署或回滚；所有者的 5173 服务保持原状。浏览器证据来自本机 Chrome 与视口/媒体模拟，实体设备、Safari、Firefox 和实际读屏软件未验。

