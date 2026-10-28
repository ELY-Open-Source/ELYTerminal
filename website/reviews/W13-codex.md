FAIL

W13 第 1/3 轮。浏览器行为验收通过；唯一阻塞是新增测试注释超过本轮明确规定的十词上限。

**F1：`website/tests/stage.mjs:145` 的注释为 11 词。**

该行按空格分词为 11 个英文词，超过要求的 10 词。根因是新增 context lost 回归时写入了超长注释；影响为本轮仓库规则验收失败，页面行为测试均通过。

复现，在仓库根目录运行：

```sh
python3 -c 'from pathlib import Path; s = Path("website/tests/stage.mjs").read_text().splitlines()[144]; print(s); print(len(s.removeprefix("// ").split()))'
```

输出词数为 `11`。最小修复：将此行缩为 `// A lost context leaves the themed page visible.`（8 词）。

证据：[当前源码摘录截图](../../target/website-review-w13/run-3bjbtinv/shots/comment-rule.png)、[逐文件注释检查](../../target/website-review-w13/run-3bjbtinv/gates.json)。截图从本轮副本的实际文件读取该行并显示计数。

**两处功能修复已独立确认。**

页眉使用 `var(--ground)` 后，320×568、844×390 的深浅主题逐段滚动截图比较通过：显示正文与临时隐藏正文时的页眉条带一致。独立重放 Sessions/Git 顶部再滚轮 400px，页眉内文字保持清楚，正文在条带下方阅读。见 [Sessions 滚动](../../target/website-review-w13/run-3bjbtinv/shots/native-sessions-400.png)、[Git 中文浅色](../../target/website-review-w13/run-3bjbtinv/shots/native-zh-light-git-400.png)。

真实 `WEBGL_lose_context` 后，`#stage` 计算样式为 `display:none`，静态 Ely 显示，页面保留当前主题。五种尺寸、深浅两色均通过背景像素检查：深色为 `(12,12,13)`，浅色为 `(241,239,233)`。此前白色失效画布问题已关闭。见 [320 深色回退](../../target/website-review-w13/run-3bjbtinv/shots/fallback-lost-320-dark.png)、[844 浅色回退](../../target/website-review-w13/run-3bjbtinv/shots/fallback-lost-844-light.png)。

**独立验收与证据。**

在 `target/website-review-w13/run-3bjbtinv/build/` 的独立副本完成干净安装、`npm --prefix website test`（32/32）、生产构建及 preferences、downloads、content、placement、stage、visual 六套浏览器测试。全部通过；构建只有已接受的 Three chunk 大小警告，帧间隔均值为 16.67ms。日志：[install.log](../../target/website-review-w13/run-3bjbtinv/install.log)、[commands.log](../../target/website-review-w13/run-3bjbtinv/commands.log)。

| 验收范围 | 独立结果 |
| --- | --- |
| 场景、布局与滚动 | 1440×900、1280×720、320×568、390×844、844×390，另含 1280×600；两语言、两主题共 24 组、120 个场景/页签采样，实际 WASM 人物单元与文字相交数、槽外单元和横向溢出均为 0。20 个场景内滚动位置、原生滚轮、页面底部及同页缩放通过。触屏模拟原生滑动后 `scrollY=501`，横向溢出为 0。 |
| 键盘与偏好 | 五尺寸 × 两语言 × 两主题，共 20 组、288 个 Tab 焦点步骤；顺序、可见焦点环、命中位置通过。Skip link、Shift+Tab、方向键、Home/End、Enter/Space 操作、保存后重载及系统主题变化通过；两主题下拒绝 localStorage 仍能操作并有明确警告。 |
| 动画 | 开场、四姿态重组、指针和按压通过。初始 reduce、实时双向切换、开场中途切换及暂缓 WASM 加载期间双向切换通过。桌面/手机、两语言的整页截图在 reduce 后稳定，打字状态清空，恢复正常动画后像素继续变化。 |
| 回退与错误 | WASM 404、WebGL 初始化失败、真实 context lost，五尺寸 × 两主题共 30 组通过；静态 Ely、主题背景、Files 页签和下载区域可用，失败有明确日志。正常路径没有 pageerror 或 console.error。 |
| 下载 | 五种平台情形的主下载映射及六个无版本别名正确；六条公开下载地址本轮独立执行 redirected HEAD，全部经两次重定向返回 200。 |

测量与脚本：[probe-summary.json](../../target/website-review-w13/run-3bjbtinv/probe-summary.json)、[keyboard.json](../../target/website-review-w13/run-3bjbtinv/keyboard.json)、[acceptance.json](../../target/website-review-w13/run-3bjbtinv/acceptance.json)、[download-head.json](../../target/website-review-w13/run-3bjbtinv/download-head.json)、[独立探针目录](../../target/website-review-w13/run-3bjbtinv/probes/)。

已目视检查八张桌面深浅主题场景截图，以及手机、短横屏、英文/中文、Files、焦点环、原生滚动和回退截图。示例：[手机英文 Sessions](../../target/website-review-w13/run-3bjbtinv/shots/en-US-dark-390x844-sessions.png)、[中文短横屏 Git](../../target/website-review-w13/run-3bjbtinv/shots/zh-CN-light-844x390-git.png)、[320px 主题按钮焦点](../../target/website-review-w13/run-3bjbtinv/shots/keyboard-320-dark-en-US-theme-toggle.png)。完整截图在 [regression-shots](../../target/website-review-w13/run-3bjbtinv/regression-shots/) 与 [shots](../../target/website-review-w13/run-3bjbtinv/shots/)。

受检手写文本文件均不超过 500 行，最大为 `style.css` 的 490 行；其余受检代码注释符合单行、十词上限。未发现新增死代码、静默回退或页面功能阻塞。编译后的 WASM 与原工作区及副本服务器文件哈希一致。

本轮浏览器只连接副本服务 `http://127.0.0.1:62730`；该服务已停止，临时构建、依赖与缓存已删除。工作区仅新增本报告，既有 751 个文件、44 张截图及原 WASM 保持一致，见 [integrity.json](../../target/website-review-w13/run-3bjbtinv/integrity.json)。用户的 5173 服务保持原样。

验证边界：Chrome 156.0.8078.48，包含视口、媒体和触屏模拟。Safari、Firefox、实体移动设备及实际读屏软件未验；未访问网站生产域名，部署属于 W14。
