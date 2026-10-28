FAIL

W14 第 1/3 轮：发现 1 项验收阻塞。线上 HTTP 检查通过；“从已提交源码全新构建并比对生产”的要求缺少可用提交。

F1：website 整体未纳入 Git，无法完成提交树构建验证。

位置：`website/package.json:1`、`website/wrangler.jsonc:1` 所在目录的 Git 跟踪状态；对应 `website/TASKS.md:18` 的部署验收。

复现：在仓库根目录执行以下只读命令：

```sh
git rev-parse HEAD
git ls-tree HEAD website
git ls-files website
git status --short website
git cat-file -e HEAD:website/package.json
```

实际结果：HEAD 为 `75ac5f0bfb8fba623d627a0f4846ab549e3e170f`；两个文件清单均为空；状态为 `?? website/`；最后一条退出 128，报告 `website/package.json` 存在于磁盘但不存在于 HEAD。`git log -1 --all -- website` 也没有记录。

根因与影响：当前构建输入只存在于未跟踪的工作区。生产文件与本次工作区快照一致，但指定 Git 提交无法重建网站，因此本轮明确要求的提交来源验证尚未完成。此项属于发布可复现性缺口。

最小修复：将本次网站源码、锁文件和部署配置纳入明确的 Git 提交，记录该提交，再从该提交创建干净副本构建并核对双域名。若提交产物仍与线上逐字节一致，可直接使用比对结果完成此项验收。

证据：`target/website-review-w14/run-nwvbyqee/provenance.txt`、`committed-tree-check.json`；后续证据路径均相对于该目录。

已通过的独立检查：

1. 隔离工作区构建：复制当前网站到审查目录，执行 `npm ci --no-audit --no-fund` 与 `npm run build`，均退出 0。使用 Rust 1.95.0、Vite 8.3.5、Wrangler 4.154.0；构建仅出现已接受的 Three chunk 大小警告。证据：`npm-ci.log`、`build.log`、`baseline.json`。

2. 双域名文件一致性：对 `https://terminal.elygpui.com` 和 `https://elyterminal.zhangyanghaha0407.workers.dev` 逐文件 GET，跟随规范化重定向并解压 HTTP 内容。全部 9 个文件、18 次比对均返回最终 200，内容与全新工作区构建的 SHA-256 完全一致；两个首页根路径也与构建的 `index.html` 一致。覆盖 HTML、两份 JS、CSS、WASM、SVG、WOFF2 和两份许可证。HTML/CSS/JS/SVG/字体/许可证的 MIME 分别正确；WASM 为 `application/wasm`，并在 Node 中直接通过两个线上 URL 执行 `WebAssembly.instantiateStreaming` 成功。证据：`file-comparison.json`、`http-check.log`、`wasm-streaming.log`、`http/` 下逐请求的响应头、正文和元数据。

3. 缺失路径与资源引用：两个域名的随机缺失普通路径、缺失 WASM 路径均返回 404；带 `Accept: text/html` 的缺失路径也返回 404。首页引用的 JS、CSS、SVG、字体，以及脚本引用的 stage chunk、WASM 均包含在已核验文件中。证据：`http/*missing*`、`references.json`。

4. 下载目标：两个线上 HTML 的六个直接下载地址一致，全部采用 `releases/latest/download/ASSET`；线上 JS 保留相同的 releases 前缀、latest/download 路径和六个别名。独立 `curl --head --location` 验证 DMG、ZIP、AppImage、DEB、RPM、tar.gz 均经两次重定向返回 200，主链接初始指向的 releases 页面返回 200。证据：`download-checks.json`、`http/download-*`。

5. 配置与版本记录：`wrangler.jsonc:5` 保留 `workers_dev: true`，`assets.directory` 为 `./dist`，唯一 route 为 `terminal.elygpui.com` 且 `custom_domain: true`。只读 `wrangler deployments list --json` 和 `wrangler versions list --json` 均列出当前版本 `66651f41-7518-454e-b9be-78998eace142` 与回滚版本 `6797e7d3-4e23-4cd5-bed2-04bf05030625`。最新部署时间为 2026-10-28 18:13:14 UTC，当前版本承接 100% 流量；旧版本注记为 W10 Aurora，TASKS 的回滚目标一致。证据：`deployments.json`、`versions.json`。

操作边界：本轮使用 HTTP 和上述两个只读 Wrangler 命令。生产浏览器、Playwright、部署及回滚实操均按授权边界省略；无关 DNS 记录的历史变更缺少部署前快照，本轮未审计。原网站源码与 5173 开发服务器保持原状；审查结束删除隔离构建目录，保留 HTTP 证据和日志。

