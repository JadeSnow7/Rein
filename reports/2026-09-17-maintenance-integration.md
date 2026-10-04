# Rein 第 13–17 章离线维护集成交付

本轮把候选补丁、调用者批准、真实 Node 宿主应用、修改后验证和最多两次修复连接起来，并补充正文的进入状态、产物与可运行练习。默认仍是只读 profile；显式启用维护 profile 才注册两个新工具。0.1 历史契约与旧数字章节主题保留。

## 实际入口

- `rust/src/rein/maintenance_session.rs`：会话顺序、最新输入、候选基线、审批与停止原因。
- `rust/src/rein/extension02_executor.rs`、`ts/src/rein/extension02-host.ts`：同一 0.2 信封中的受控验证与补丁应用。
- `rust/examples/document_maintenance.rs`：离线命令维护示例，默认展示差异并等待本地审批；只有显式 `--approve-demo` 才由演示调用者批准。
- `scripts/maintenance-fixture.mjs`：只生成任务输入，不复制评测答案，不覆盖已有目录。
- `docs/chapters/approved-patch.md`：已实际执行的完整命令；阶段汇总 4 链接此练习。

生成器从真实读取的 README 与 package.json 提议替换；它没有使用评测 `expected`。报告保留每轮输入版本、候选摘要与差异、批准对象、修改后摘要、原始验证输出、实际派发与宿主回收记录。生成期间或批准后的目标基线变化均不能静默应用。

## 检查与证据边界

针对性 Rust 检查覆盖只读协议、候选、维护 profile 和会话；TypeScript 检查覆盖维护工具。正文第 13 章的过期命令检查返回 exit 1 是预期的反例结果，第 16 章实际修改后的文件为 `npm run dev`，报告为 `offline_fixture`、`Completed`，四次宿主调用均派发并回收。

原始记录位于 `records/REIN-BOOK-V2-20260917/evidence/`：

- `maintenance-runtime-final-v2.json`：最终隔离实现的针对性 Rust 执行。
- `maintenance-ts-final-v1.json`、`maintenance-typecheck-final-v1.json`：TypeScript 检查。
- `lesson-13-negative-v1.json`：正文负例，exit 1 且验证 JSON 的 ok 为 false。
- `lesson-16-demo-v1.json`、`lesson-16-review-v1.json`：正文命令、实际报告与修改后文件哈希核对。
- `target-book-check-a2.json`、`target-build-a2.json`、`target-links-a2.json`：原目录结构、构建和 3944 个站内目标/锚点检查。
- `target-language-routes-v1.json`：语言路由与锚点映射的六项静态检查。

保留了早先失效或失败记录。一处旧维护测试使用时间戳目录且忽略创建冲突，本轮改为独立序号和排他创建；不通过关闭并行掩盖问题。虚假宿主测试曾在 ready 前因 ESM 语法错误退出，修正后要求实际派发、完整伪回执与磁盘不变，才能验证 core 拒绝伪写入成功。

## 仍未完成的交付

本轮是当前源码的离线集成，不是逐章快照验收。00–17 的输入/输出快照、从前驱恢复的跟做、真实模型生成、指定章节提示词试用、24 的基础评测及 18–23 的完整分支仍待完成。章节状态维持 draft，actual 为 false，快照仍为 null。

维护 profile 对受控小夹具做全文件版本核对，不宣称大项目的扫描性能、操作系统沙箱或任意 shell 安全性。CLI 的模型、token、成本与模型延迟字段保持 null。宿主的进程回收证据限定于实际测试路径，不等于所有崩溃与后代进程场景均已覆盖。

本地代码与正文回写沿用当前授权，逐文件核对原目标基线。未执行提交、推送、创建 Git 标签或发布。
