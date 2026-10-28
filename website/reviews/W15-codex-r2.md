PASS

W15 第 2/3 轮。首轮唯一阻塞 F1 已关闭；本次修复的独立复现与相关回归通过。

审查对象为当前工作区。逐文件 SHA-256 对比首轮保存的 755 个文件，唯一变化是 `website/package.json:8` 的 dev 脚本增加 `--host 127.0.0.1`；另新增首轮报告。AGENTS.md、check.sh、PRD、TASKS、网站实现、测试和锁文件均与首轮快照一致。证据：`target/website-review-w15-r2/run-4mzprt2v/source-comparison.json`、`package-script-diff.txt`。

F1 关闭证据：

在 `target/website-review-w15-r2/run-4mzprt2v/copy/` 中干净执行 `npm --prefix website ci`，随后保留 package.json 原样，仅指定自用端口：

```sh
npm --prefix website run dev -- --port 50410 --strictPort
```

命令先成功构建 WASM，再执行 `vite --host 127.0.0.1 --port 50410 --strictPort`。Vite 打印 `http://127.0.0.1:50410/`；`lsof` 明确显示 IPv4 `127.0.0.1:50410 (LISTEN)`。使用 `curl --noproxy '*'` 访问 `/`、`/src/main.js`、`/src/stage.js`、`/src/swarm.wasm`、`/@vite/client` 和本地字体，均退出 0、返回 HTTP 200。WASM 的 Content-Type 为 `application/wasm`，响应字节与本次生成文件一致。证据：`dev-server.log`、`dev-listener.txt`、`http-probes.json`。

因此 `AGENTS.md:35` 的 IPv4 地址与脚本默认绑定一致；浏览器套件所用地址保持可达。自用端口用于避开所有者的 5173，脚本没有额外 host 参数或配置覆盖。

相关回归：

| 检查 | 独立结果与证据 |
| --- | --- |
| 开发模式启动与资源加载 | 本机 Chrome 访问同一 IPv4 服务；Vite WebSocket 成功连接，10080 粒子的 swarm 与 stage 均输出 ready，页面 WebGL/swarm 状态为 ready，字体 loaded。pageerror、console error、失败请求均为空。`browser-probe.json`、`dev-ready.png`。 |
| 控制、存储与布局 | `BASE_URL=http://127.0.0.1:50410 node website/tests/preferences.mjs` 退出 0；英文/中文、系统主题变化、手动选择及重载持久化、键盘、拒绝 localStorage、六视口布局通过。`preferences.log`、`shots/`。 |
| 生产构建 | `npm --prefix website run build` 退出 0；9 个文件逐一 SHA-256 与首轮构建完全一致。仅保留已接受的 Three chunk 大小警告。`build.log`、`build-comparison.json`。 |
| 仓库 gate | 在副本按当前 756 个文件建立隔离索引，提取原 `scripts/check.sh` 的行数与 Rust 连续行注释检查，以 `sh` 执行，退出 0。两个锁文件的文件名豁免保持原样。`house-gate.sh`、`gate.log`、`gate-result.json`。 |
| 网站文件与注释 | 网站自有手写文件最大为 style.css 的 490 行；扫描到的 26 条实际注释均为单行、最多十词。本次修改只增加启动参数，运行时模块与调用关系保持原样。`comments.json`。 |

首轮 AGENTS.md 的 Stack、Commands、Layout、下载别名决策，以及 PRD 对照结论继续成立。网站运行时代码与生产产物均未变化，本轮没有新增需求、正确性或可访问性阻塞。W11 继续保留 TASKS.md 所记的三轮封顶、待所有者处理状态。

证据统一保存在 `target/website-review-w15-r2/run-4mzprt2v/`。自建服务器已停止，50410 端口监听已消失；隔离副本中的构建产物和依赖已删除。原工作区既有文件哈希保持不变，仅新增本报告。见 `server-stop.json`、`integrity.json`。

验证边界：本轮针对唯一的 dev 绑定变更，复跑 preferences 套件与启动探针；首轮其余五套浏览器检查、单元测试及全场景目视结论沿用其已保存证据。未运行原生应用 cargo test、生产浏览器、生产 HTTP、部署或回滚；所有者的 5173 服务保持原状。
