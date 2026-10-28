PASS

W7 第 3/3 轮复审，2026-10-28。已重新读取 TASKS.md、PRD.md 和前两轮报告。r2 的 F2 已关闭，r1 的 F1 和直接短屏布局回归继续通过。本轮代码、测试、布局及发布脚本范围内，未发现可复现的剩余验收阻塞。

**F2 关闭：同页缩窗重算姿态**

`website/src/ely.js:134` 新增 window resize 监听，调用既有 measure()；`:132–133` 保留 main 的 ResizeObserver。measure() 更新场景锚点、视口相关姿态及 dirty 状态，由原有 rAF 调度器应用。与 r2 源码副本比较，运行时改动只有这一行；`website/tests/downloads.mjs:84` 起新增同页缩窗回归。

独立探针在同一页面依次使用 1280×720、1280×600、1280×720、844×390、1280×600，调整尺寸后在页面右侧发送鼠标滚轮事件至底部，等待 1300ms。正常动画/深色与减弱动画/浅色各执行一轮，全部通过。每一步均检查实际 scrollY、页面宽度、Ely 图形和外层容器与标题/操作区的相交，以及操作区底部可见性。

| 同页状态 | main 高度 | 实际姿态 | 结果 |
|---|---:|---|---|
| 初始 1280×720 | 2880px | translate3d(461px,72px,0px) scale(1.06) | 标题和操作区与 Ely 分离。 |
| 缩至 1280×600 | 2880px | translate3d(25.6px,0px,0px) scale(0.8) | 缓存得到更新，标题和操作区与 Ely 分离。 |
| 恢复 1280×720 | 2880px | translate3d(461px,72px,0px) scale(1.06) | 恢复对应姿态，布局通过。 |
| 调整至 844×390 | 2880px | translate3d(16.88px,0px,0px) scale(0.8) | 短屏布局通过。 |

1280×600 下 scrollY=2280px，场景锚点从 `[0,720,1440,2160]` 变为 `[0,704,1408,2112]`。Ely 图形范围为 x=106.71–305.03、y=31.13–312.76；标题为 x=394.27–885.73、y=281–379.55；操作区为 x=435.66–844.34、y=457–566.78。标题与 Ely 水平分离，操作区完整位于 600px 视口内。

另做根因对照：仅在独立测试浏览器的模块响应中移除新增监听，保持仓库和服务器文件原样。同页缩窗后立即复现 r2 的 `translate3d(461px,72px,0px) scale(1.06)` 与标题重叠；保留监听的当前实现通过相同断言。已查看用户提供的 download-resize-bottom.png，以及本轮正常、减弱动画、移除监听对照的三张截图，结果与矩形数据一致。

证据：`target/website-review-w7-r3/run-iamxpkb4/website/resize-review.mjs`、`resize-review.json`、`resize-current-600.png`、`resize-reduced-600.png`、`resize-listener-removed-600.png`。最后一张仅展示测试浏览器中的旧行为对照。

**F1 与下载布局回归**

独立平台探针再次使用完整 desktop Safari UA、MacIntel、userAgentData=undefined，对照 maxTouchPoints=5 和 0：前者返回 other，主按钮为 View releases 并指向 releases 页面；后者返回 macos，主按钮指向稳定 DMG。Windows、Linux、移动 UA 的 iPad、iPhone、Android、ChromeOS、FreeBSD 对照均通过，F1 保持关闭。

独立下载布局检查覆盖 1440×900、1280×720、390×844、320×568、844×390、1280×600。页面宽度与视口一致，标题及七个链接可见，链接互相分离且点击命中正确。六个格式链接均匹配规定的 versionless URL；点击在浏览器中阻止导航并记录 href。直接短屏加载由现有 downloads.mjs 继续覆盖，真实同页缩窗由上述独立探针覆盖。

**独立执行与发布脚本**

安装、构建和测试在指定临时目录的源码副本执行；浏览器连接原开发服务器。全部命令退出 0。

| Command | Result |
|---|---|
| `npm --prefix website ci --no-audit --no-fund` | 干净安装通过。 |
| `npm --prefix website test` | 6/6 通过。 |
| `npm --prefix website run build` | 14 个模块；JS 534.38 kB / gzip 134.47 kB，仅保留已接受的 Three chunk 警告。 |
| `node website/tests/visual.mjs` | 六视口通过，字体 loaded，errors / failures 为空。 |
| `node website/tests/ely.mjs` | 八张姿态捕获与减弱动画吸附通过。 |
| `node website/tests/content.mjs` | 六视口、双语数据、鼠标/键盘页签及 W6 窗格末尾可达性通过。 |
| `node website/tests/scene.mjs` | 深浅主题、八张场景捕获、减弱动画及 WebGL 回退通过；平均帧间隔 16.67ms。 |
| `node website/tests/downloads.mjs` | 五组平台输入、六链接、六视口和新增同页缩窗断言通过。 |

发布脚本及 packaging/linux/README.md 与 r2 逐字节相同。再次使用本地 cargo/gh 桩和另一版本 2.4.6 验证六格式：普通上传保留版本化资产并上传别名，alias-only 只上传别名，共 12 个映射与 payload SHA-256 检查通过。四个参数/未知别名拒绝场景在上传前失败；三个上传故障场景非零退出并清理临时目录。发布仍使用现有 release-asset.sh，六个稳定名称及 README 中的行为说明保持有效。

日志及数据位于 `target/website-review-w7-r3/run-iamxpkb4/`：`commands.log`、`commands.json`、`independent-downloads.json`、`release-probe.json`。新生成的完整回归截图在该目录的 `target/website-shots/`。

**约束与边界**

修复涉及的 ely.js 为 151 行，downloads.mjs 为 107 行，均无源码注释。重点手写文件均低于 500 行。修复保持现有单一 rAF、原生滚动及 SVG 更新路径；运行时下载链接保持版本无关。页面内容、CSS、下载映射、调度器及发布实现与 r2 相同，未引入营销嵌套卡片、额外文案或发布系统。W1–W6 的上述回归全部通过。

| Input | SHA-256 |
|---|---|
| `website/src/ely.js` | `0d5ae12a916d00efbf40388f4b19440e2d7d737e4ea5c36b8b3003aded63ce1b` |
| `website/tests/downloads.mjs` | `54051f2239d6bbf492588c205d6a33daa8c912a5f2c85fd9631df26937c2cfa9` |
| `scripts/release-asset.sh` | `9d98069dec5a0c2eb3598b29478c5311095c44ebd39ee7bc0d5ee48fd4b83c08` |

除指定临时目录外，本轮仅新增本报告。起始文件及 41 张原截图的 SHA-256 已复核；实现、公开 release、远端资源及开发服务器保持原样，独立浏览器已关闭。

公开别名上传与 redirected HEAD 验证待所有者授权后进行，本轮未验。实体 iPad、原生 Safari/Firefox 和生产下载未测试；平台判断来自 Chrome 属性仿真，布局证据来自 Chrome 156.0.8078.48。本 PASS 限于本轮授权的代码、测试、布局及发布脚本范围。
