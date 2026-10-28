FAIL

W1 审查完成；确认 1 个阻塞项：下载契约缺少固定资产名的发布前置条件。

范围：`website/PRD.md`、`website/TASKS.md`，以及本次请求给出的架构约束。审查日期：2026-10-28。仓库 HEAD：`75ac5f0bfb8fba623d627a0f4846ab549e3e170f`。两份输入文档当时均为未跟踪文件，以下 SHA-256 标识本次审查内容。

| Input | SHA-256 |
|---|---|
| `website/PRD.md` | `96d9b93bf00bc60286157a3ee8bdb68952b8dc69f21e9aec7a58a52e9b573bb9` |
| `website/TASKS.md` | `f26761f11e3d9ca68c0174d7fb142609a7b09217f9b33ecd981a0ee98789357c` |

**F1 · P1 · 固定下载地址依赖尚未发布的固定资产名**

位置：`website/PRD.md:21`、`website/PRD.md:25`、`website/TASKS.md:5`、`website/TASKS.md:11`。

PRD 同时要求当前版本的直接下载、`releases/latest/download/ASSET`、禁止硬编码版本，以及纯静态站点和禁止远程运行时 API。GitHub 的该地址按最新 release 中的完整资产名查找文件；`latest` 只负责选择 release，资产名仍需精确匹配。[GitHub 官方下载链接规则](https://docs.github.com/en/repositories/releasing-projects-on-github/linking-to-releases)。

本次实时读取 [latest release API](https://api.github.com/repos/ELY-Open-Source/ELYTerminal/releases/latest)，返回 [v0.1.9](https://github.com/ELY-Open-Source/ELYTerminal/releases/tag/v0.1.9)，共六个上传资产，全部携带版本号。下表对每个名字使用同一前缀 `https://github.com/ELY-Open-Source/ELYTerminal/releases/latest/download/`，执行匿名 HEAD 并跟随重定向；版本号仅用于记录已验证的发布事实。

| Format | Published asset | Final HTTP | Version-stripped probe | Final HTTP |
|---|---|---|---|---|
| macOS DMG | `ELYTerminal-0.1.9.dmg` | 200 | `ELYTerminal.dmg` | 404 |
| Windows x86_64 ZIP | `ELYTerminal-0.1.9-windows-x86_64.zip` | 200 | `ELYTerminal-windows-x86_64.zip` | 404 |
| Linux x86_64 AppImage | `ELYTerminal-0.1.9-linux-x86_64.AppImage` | 200 | `ELYTerminal-linux-x86_64.AppImage` | 404 |
| Linux amd64 DEB | `elyterminal_0.1.9_amd64.deb` | 200 | `elyterminal_amd64.deb` | 404 |
| Linux x86_64 RPM | `elyterminal-0.1.9-1.x86_64.rpm` | 200 | `elyterminal-1.x86_64.rpm` | 404 |
| Linux x86_64 tarball | `ELYTerminal-0.1.9-linux-x86_64.tar.gz` | 200 | `ELYTerminal-linux-x86_64.tar.gz` | 404 |

右侧名字仅为去掉发布版本后的探测样本；正式固定名字需要在 W1 锁定。成功地址均为 `302 → 302 → 200`，最终响应带对应文件的 `Content-Disposition: attachment`；失败样本均为 `302 → 404`。本地 `scripts/release-asset.sh:9` 按传入文件名上传，`scripts/mac-app.sh:60` 和 `scripts/windows-zip.ps1:11` 生成带版本号的名字。

影响：直接采用现有资产名会把版本号固定在网站链接里；未来最新 release 改用新版本文件名时，该地址会失效。采用当前不存在的固定名字会立即返回 404。W7 的一次 HEAD 检查可以让带版本号的当前地址通过，因此现有验证项也需要覆盖跨版本保持相同 URL 的要求。W1–W11 当前没有安排固定名字的发布责任与依赖。

修正与复验：

1. 在 W1 明确六种格式各自的固定资产名，并把“当前及后续每个最新 release 都提供这些名字”列为发布前置条件；该前置条件须在 W7 的下载验收前满足。
2. 保持既定静态架构，为当前及后续 release 提供对应安装包的固定名称资产；网站使用这些确定的 `releases/latest/download/ASSET` 地址。
3. W7 对六个正式固定地址执行跟随重定向的 HEAD，检查最终 200 和目标文件；同时验证地址中的资产名独立于版本，后续 release 的发布契约保留同一组名字。

本次仅提出上述修正要求，GitHub release 与发布脚本保持原状。

**其余要求、架构与任务契约**

| Review area | Result |
|---|---|
| 场景与内容 | PRD:5、13 明确单一连续场景，顺序为 terminal → sessions/live command state → Git/Files → downloads；W3、W6、W7 分别承接。 |
| 渲染与滚动 | 按本次请求，Vite + vanilla JS modules；Three.js 仅负责全视口 synthwave 背景；inline layered SVG 负责 Ely 的 pose rig；原生滚动配合 requestAnimationFrame 插值。PRD:17、25 和 W2、W4、W5 与此相容，职责范围足够完成需求。原始 SVG 已有 eyes、face、terminal、fingers 等分层，支持复用原路径。 |
| 语言、主题与可用性 | PRD:5、9、17、29 覆盖 English/简体中文、系统主题随动至用户选择、设计过的明亮配色、reduced motion、无 WebGL 时可读与键盘操作；W4、W5、W8、W9 承接，W8 明确选择持久化。 |
| 顺序与验收 | W2 构建基线 → W3–W6 场景与运动 → W7 下载 → W8 语言/主题控制 → W9 综合验收 → W10 部署复验 → W11 文档与 500 行检查。PRD:29 的平台/链接/locale/theme 单元检查、逐场景截图和浏览器检查共同约束各任务；TASKS:3 的逐项通过审查与最多三轮规则适用于全表。F1 是该顺序中需要补齐的发布依赖。 |
| 复杂度 | 技术选择对应已请求的背景、角色、滚动与静态发布；计划保持小型模块、原有 SVG 和浏览器能力，未发现具体过度设计。 |

部署方案与 [Cloudflare Static Assets](https://developers.cloudflare.com/workers/static-assets/) 相容：Wrangler 上传 Vite 构建产物，Cloudflare 直接提供静态文件。目标 `terminal.elygpui.com` 使用 [Custom Domain](https://developers.cloudflare.com/workers/configuration/routing/custom-domains/)。PRD:33 的 `workers.dev` 保留要求需要在部署配置中显式启用 `workers_dev: true`；Cloudflare 文档说明配置 routes 时会把省略的该选项推断为 false。[workers.dev 配置规则](https://developers.cloudflare.com/workers/configuration/routing/workers-dev/)。W10 已承担部署、生产复验和回滚记录；账号、zone 与 DNS 的实际状态留待该任务验证。

**禁止项核对**

| Constraint | Confirmed plan state |
|---|---|
| Nested cards | PRD:5、W3 明确排除嵌套卡片，连续场景承担内容。 |
| Taglines / descriptions | PRD:5 明确排除 tagline、feature descriptions、secondary labels、decorative badges；PRD:13 只允许产品名或可见功能名作为大标题。 |
| Backend / auth / database | PRD:25 明确静态资产，排除 backend、database、accounts 和 remote runtime API；任务表没有服务端或认证工作。 |
| Hardcoded release versions | 两份文档的发布版本字面量扫描结果为空，PRD:21 明确禁止硬编码版本。下载地址的可执行性受 F1 阻塞。 |

实际执行：完整读取两份 W1 文档，核对仓库规则、原始 SVG 分层和相关发布命名代码，读取四份官方文档与当前 release API，并对六个真实资产地址及六个去版本号样本执行 HEAD。两份输入文档分别为 39 和 15 行。

未执行网站构建、浏览器/UI、性能、无 WebGL 或生产部署测试；网站当前仅有规划文档，运行验证属于 W2–W11。安装包内容与二进制兼容性未复验。本次风险为 F1 所述固定下载契约尚未具备发布前置条件。

