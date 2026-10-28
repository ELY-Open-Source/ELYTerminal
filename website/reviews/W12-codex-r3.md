PASS

W12 第 3/3 轮。r2 的 F1 已关闭，本次位置修复的回归检查通过。

**F1 已关闭：正文阅读过程中的 Ely 保持在当前场景的预留槽位。**

`website/src/stage.js:151` 的 `place()` 每帧读取 `slots[current]` 的实时矩形，`:229` 按 0.58 迟滞切换当前场景，`:230` 将该槽位交给 `figure()`。跨槽位插值 `placeAt` 及其旧单元测试已删除；正常动画的场景转换仍通过粒子飞行完成。

独立重放 r2 原路径：320×568、英文深色、正常动画，滚至 Sessions/Git 顶部，再用原生鼠标滚轮下滚 400px，停止并等待 3 秒。读取真实 WASM 输出中的有效 Ely 单元，排除环境尘埃，并与可见文字 Range 和当前槽位比较。

| 路径 | 当前实测 |
| --- | --- |
| Sessions +400px | `scrollY=1038`；4980 个 Ely 单元；文字相交数 0，槽外单元数 0 |
| Git +400px | `scrollY=1821`；4854 个 Ely 单元；文字相交数 0，槽外单元数 0 |
| 同一 Git 位置切换中文、浅色 | 文字相交数 0，槽外单元数 0；正文保持可读 |
| reduce 与正常动画逐段阅读 | 两场景各采样 +0/+100/+200/+300/+400px，共 20 个位置，均为 0 相交、0 槽外、0 横向溢出 |

当前槽位随原生滚动离开视口时，Ely 随槽位离开；稳定后的阅读区域没有残留人物覆盖。已目视核验 [Sessions 原始滚轮路径](../../target/website-review-w11-r3/run-vqll668d/shots/native-sessions-400.png)、[Git 原始滚轮路径](../../target/website-review-w11-r3/run-vqll668d/shots/native-git-400.png)、[中文浅色](../../target/website-review-w11-r3/run-vqll668d/shots/native-zh-light-git-400.png) 和 [reduce](../../target/website-review-w11-r3/run-vqll668d/shots/scroll-reduce-git-400.png)。数据见 [native.json](../../target/website-review-w11-r3/run-vqll668d/native.json)、[scroll.json](../../target/website-review-w11-r3/run-vqll668d/scroll.json)。

**像素、布局及交互回归。**

独立运行新增 `placement.mjs`：320×568、844×390、1280×720 → 844×390 同页缩放，英文和中文，每 140px 采样直至页面底部；隐藏正文后测量实际粉色像素范围，槽位半数以上可见时检查 Ely 出现；全部通过。正常动画下 Sessions/Git 原生滚轮 +400px、等待 3 秒的像素检查也通过。

另以真实 WASM 输出完成六视口 × 两语言 × 两主题共 24 组、120 个场景/页签采样，人物与文字相交、槽外单元和横向溢出均为 0。`content.mjs` 保留的文字行框与槽位相交矩阵通过。独立检查 Home 键切回 Git；现有浏览器套件的键盘、偏好持久化、系统变化、存储拒绝和六个下载链接回归通过。

844×390 实际滚至页面底部，及 1280×720 同页缩至 1280×600 后滚到底部，下载标题、主链接和六种格式可见，人物与文字相交数为 0。见 [短屏底部](../../target/website-review-w11-r3/run-vqll668d/shots/download-844-bottom.png)、[同页缩放底部](../../target/website-review-w11-r3/run-vqll668d/shots/download-resize-bottom.png)。已目视检查八张桌面深浅主题场景截图，以及窄屏、横屏、Files 与上述独立复现截图。

target 临时副本的干净安装、32/32 单元测试、生产构建，以及 preferences/downloads/content/placement/stage/visual 六套浏览器测试全部通过。构建仅有已接受的 Three chunk 大小警告；stage 平均帧间隔 16.67ms。正常路径没有 pageerror 或 console.error。命令输出见 [commands.log](../../target/website-review-w11-r3/run-vqll668d/commands.log)。

相关手写实现与测试均不超过 500 行，最大为 485 行；注释均为一行且不超过十词。旧插值及其调用已删除，本次修复没有新增营销嵌套卡片、填充文案、玻璃样式、死代码或版本化 API。context lost 后失效画布的回退问题记录于 [W11 本轮 F2](W11-codex-r3.md)。

证据保存在 [本轮目录](../../target/website-review-w11-r3/run-vqll668d/)。临时构建目录、安装依赖和缓存已删除；仓库仅新增本轮两份报告，既有 749 个文件、87 张截图及原 WASM 保持一致，见 [integrity.json](../../target/website-review-w11-r3/run-vqll668d/integrity.json)。

验证边界：本机 Chrome 156.0.8078.48 与视口、媒体模拟；Safari、Firefox、实体设备和实际读屏软件未验。始终使用已有开发服务器，未访问生产站点。
