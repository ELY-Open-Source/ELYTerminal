PASS

W1 第 2/3 轮审查通过。F1 已在规划与契约层解决；剩余具体阻塞：0。

范围：重新完整读取 `website/PRD.md`、`website/TASKS.md`，逐项复核 `website/reviews/W1-codex.md` 的 F1，并沿用本次任务已指定的架构边界。审查日期：2026-10-28。仓库 HEAD：`75ac5f0bfb8fba623d627a0f4846ab549e3e170f`。

| Input | SHA-256 |
|---|---|
| `website/PRD.md` | `a85201ad98b59dc37aabb2f8015ef143b7977295bac7ef3fe70ee04935afe170` |
| `website/TASKS.md` | `a38f5aeb62daa58e0811ada211d590056f0c2eb1729b8616dce2cb7a3dee3c53` |

**F1 复验**

原始根因是固定下载 URL 缺少可执行的固定资产名发布契约。现在 `website/PRD.md:23` 锁定以下六个名字，覆盖全部请求的发布格式：

| Format | Exact alias |
|---|---|
| macOS Apple silicon DMG | `ELYTerminal-macos-arm64.dmg` |
| Windows x86_64 ZIP | `ELYTerminal-windows-x86_64.zip` |
| Linux x86_64 AppImage | `ELYTerminal-linux-x86_64.AppImage` |
| Linux amd64 DEB | `elyterminal_amd64.deb` |
| Linux x86_64 RPM | `elyterminal.x86_64.rpm` |
| Linux x86_64 tarball | `ELYTerminal-linux-x86_64.tar.gz` |

`website/PRD.md:21–23` 将这些名字与 `releases/latest/download/ASSET` 绑定，要求当前和未来每个 release 在带版本号的文件旁提供同一组六个别名，并明确先完成当前别名上传和发布脚本接入，再检查网站链接。完整资产名保持固定，满足 [GitHub 的 latest 下载链接规则](https://docs.github.com/en/repositories/releasing-projects-on-github/linking-to-releases)。

`website/TASKS.md:11` 将当前别名发布、未来发布脚本中的保留责任、平台检测和下载链接统一交给 W7；验收包含 unit tests、redirected HEAD requests 和 Codex review。PRD 的当前及未来发布要求共同约束该任务，首轮指出的责任与顺序缺口均已覆盖。

本轮再次读取 [latest release API](https://api.github.com/repos/ELY-Open-Source/ELYTerminal/releases/latest)，确认当前仍为 [v0.1.9](https://github.com/ELY-Open-Source/ELYTerminal/releases/tag/v0.1.9)，六个上传资产仍使用带版本号的文件名。固定别名的实际发布和脚本实现是 W7 的执行项；W1 已明确该前置条件与归属。

**其余 W1 契约复核**

| Area | Result |
|---|---|
| 架构 | Vite + vanilla JS modules；Three.js 仅拥有全视口 synthwave 背景；inline layered SVG 拥有 Ely pose rig；原生滚动配合 requestAnimationFrame 插值；Cloudflare Workers 提供静态资产。PRD:17、27 与 W2、W4、W5 继续兼容这一既定分工。 |
| 请求行为 | 连续场景顺序、sessions/live command state、Git/Files、全部下载格式、English/简体中文、主题随系统直至用户选择、选择持久化、reduced motion 和无 WebGL 可读性均有 PRD 要求及后续任务归属。 |
| 顺序与验证 | W2 构建，W3–W6 场景与运动，W7 下载及发布依赖，W8 语言/主题控制，W9 综合验收，W10 部署复验，W11 文档与 500 行检查。PRD:31 的单元、浏览器、逐场景截图及生产复验要求继续适用；TASKS:3 保留逐项通过审查和最多三轮的门槛。 |
| 部署与复杂度 | PRD:27、35 保持静态资产、Wrangler、`terminal.elygpui.com` Custom Domain 与保留 `workers.dev` 的边界；W10 负责生产验证和回滚记录。规划中未发现剩余具体矛盾、遗漏行为或过度设计。 |

**禁止项确认**

PRD:5、13 和 TASKS:7 保持单一连续场景，明确排除 nested cards、taglines、feature descriptions、secondary labels 和 decorative badges。PRD:27 排除 backend、database、accounts 和 remote runtime API；任务表中没有 auth 工作。两份文档的发布版本字面量扫描结果为空，六个别名均独立于版本；PRD:21 继续明确禁止硬编码发布版本。

实际执行：两份文档与首轮 F1 对照复核；六个精确别名的数量、唯一性、格式覆盖和版本独立性检查；发布前置条件、W7 责任及验收措辞检查；当前 release API 只读核对。输入文档分别为 41 和 15 行。

未执行网站构建、浏览器/UI、性能、生产部署或本轮下载 HEAD 测试。别名发布、发布脚本实现和实际下载可用性待 W7 验收；本次 PASS 的范围为 W1 规划契约。

