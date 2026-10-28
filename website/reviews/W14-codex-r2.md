PASS

W14 第 2/3 轮：F1 已关闭，本轮未发现阻碍验收的问题。

提交来源已独立核验：本地 HEAD 与 GitHub `ELY-Open-Source/ELYTerminal` 的 `refs/heads/main` 均为 `76fb21d3222656fdcbd0c77da97515f1f017557a`。该提交包含 `website/`，其 tree 为 `0f9e99ca335f3246e94c8ef5efb6f4a16bda18e4`；网站的 40 个非审查文件与首轮快照逐字节一致。

全部临时文件与证据位于 `target/website-review-w14-r2/run-zfcintxv/`，下文证据路径相对于该目录。

干净构建：使用 `git archive --format=tar --output=…/source.tar 76fb21d3222656fdcbd0c77da97515f1f017557a` 导出整个提交，再解压到新建的 `build/`。构建前确认归档中没有 node_modules、dist、swarm/target 或生成的 src/swarm.wasm。在 `build/website/` 执行 `npm ci --no-audit --no-fund` 和 `npm run build`，均退出 0。Rust 1.95.0 从源码编译 WASM，Vite 8.3.5 生成生产文件；构建仅出现此前接受的 Three chunk 大小警告。证据：`source.tar`、`provenance.json`、`source-manifest.json`、`npm-ci.log`、`build.log`。

逐文件 HTTP 比对：通过 curl GET、跟随重定向并解压 HTTP 内容，对照提交构建产物计算 SHA-256。`https://terminal.elygpui.com` 与 `https://elyterminal.zhangyanghaha0407.workers.dev` 的全部 9 个文件，共 18 次比对均最终返回 200，字节完全一致。两个域名的根路径首页也与构建的 index.html 一致。

| 构建文件 | 两个域名的 Content-Type | 两个域名的字节比对 |
| --- | --- | --- |
| index.html | text/html | 一致 |
| assets/index-CltxXuxL.js | text/javascript | 一致 |
| assets/stage-8-WmHlrH.js | text/javascript | 一致 |
| assets/index-D4JZR-8A.css | text/css | 一致 |
| assets/swarm-xTz8qK43.wasm | application/wasm | 一致 |
| ely.svg | image/svg+xml | 一致 |
| fonts/jetbrains-mono.woff2 | font/woff2 | 一致 |
| fonts/jetbrains-mono-LICENSE.txt | text/plain | 一致 |
| three-LICENSE.txt | text/plain | 一致 |

证据：`file-comparison.json` 保存全部本地与远端 SHA-256；`http/` 保存响应头、正文和 curl 元数据；`http-check.log` 保存检查结果。

HTTP 回归通过：两个域名均对带 `Accept: text/html` 的随机缺失路径及缺失 WASM 路径返回 404。首页引用的脚本、样式、图标和字体，以及脚本引用的 stage chunk、WASM 均解析到已核验文件。Node 直接获取两个线上 WASM URL 并执行 `WebAssembly.instantiateStreaming`，均成功。证据：`references.json`、`http/*missing*`、`wasm-streaming.log`。

下载回归通过：两个线上 HTML 包含一致的六个 `releases/latest/download/ASSET` 地址，线上 JS 的前缀和别名与之吻合。六个别名分别为 `ELYTerminal-macos-arm64.dmg`、`ELYTerminal-windows-x86_64.zip`、`ELYTerminal-linux-x86_64.AppImage`、`elyterminal_amd64.deb`、`elyterminal.x86_64.rpm`、`ELYTerminal-linux-x86_64.tar.gz`。独立 redirected HEAD 检查均经两次重定向返回 200；初始主下载链接的 releases 页面也返回 200。证据：`download-checks.json`、`http/download-*`。

部署记录复核通过：只读 Wrangler 4.154.0 的 `deployments list --json` 与 `versions list --json` 均包含当前版本 `66651f41-7518-454e-b9be-78998eace142` 和回滚目标 `6797e7d3-4e23-4cd5-bed2-04bf05030625`。最新部署仍由当前版本承接 100% 流量，与 `website/TASKS.md:18` 一致。归档配置保留 `workers_dev: true`、`assets.directory: ./dist`，唯一 route 为 `terminal.elygpui.com`，使用 `custom_domain: true`。证据：`deployments.json`、`versions.json`、`remote-main.txt`。

本轮只新增本报告，既有仓库文件经哈希复核保持原状；5173 开发服务器保持原状。隔离构建目录已清理，源码归档与验证证据保留。完整性记录：`integrity.json`。

授权与验证边界：生产检查全部通过 HTTP 完成，生产浏览器及 Playwright 按要求省略；部署、回滚及 Cloudflare 设置变更均未执行。本轮未重跑本地浏览器套件，构建输入及线上产物的一致性已独立验证。

