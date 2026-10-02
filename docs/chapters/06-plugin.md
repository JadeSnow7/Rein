---
prev: { text: 06 循环控制（Rust core）, link: /chapters/06-rust.html }
next: { text: 07 上下文与状态（Rust core）, link: /chapters/07-rust.html }
---

# 06 Plugin：让 Rust core 把一次读取交给 Node

**状态：离线实验已核验｜限时免费**

第 06 章已经在 Rust loop 中处理回合、工具预算和取消。这里继续使用同一个 Rust loop，只把早期 TypeScript 版本已经具备的只读 `read_file` 工具放到一个 Node 进程中。Rust core 保留任务状态、预算、权限和证据判断，Node 只完成受限的工具工作。

## 1、准备当前工作区和输入文件

所有命令从仓库根目录运行：

```bash
npm ci
cargo test --locked --manifest-path rust/Cargo.toml
npm test
```

确认正常输入：

```bash
export REIN_HYBRID_ROOT="$PWD"
cat fixtures/hybrid-marker.txt
```

生产 Node host 只调用已有只读路由，不写项目文件；故障场景另建临时 workspace。

## 2、运行 Rust core 到 Node host

```bash
cargo run --quiet --locked --manifest-path rust/Cargo.toml --example hybrid_stdio -- normal
```

这不是在线模型调用。示例中的 `FixtureModel` 仍按两次模型调用组织真实 loop：第一次返回带 `call-1` 的 `read_file`，core 通过 stdio host 执行读取；第二次从 `toolCallId: "call-1"` 的工具消息取出真实文件内容并生成答案。正常结果应包含：

```json
{"status":"completed","reason":"final_answer","modelCalls":2,"answer":"fixture final: ..."}
```

退出码 0 表示离线示例得到最终答案，答案内容来自文件。

源码职责也按这个顺序分开：`rust/src/rein/loop.rs` 的 `run_agent_loop_with_executor` 管理 budget、messages 和模型回合；`rust/src/rein/stdio_executor.rs` 的 `StdioExecutor` 负责 spawn、frame、cancel、evidence 与子进程清理；`ts/src/hybrid-host.ts` 只委托 `ts/src/rein/readonly.ts` 的只读路由，不复用 TypeScript loop。普通读写有默认 5 秒上限，取消后有独立 1 秒 grace；最终还要完成 EOF、exit 0 和回收检查。

## 3、读取完整 wire 身份

Node 先发送 `ready`，包含协议版本、`read_file` 能力和正数 `maxMessageBytes`。core 再发送一次 `invoke`：

```json
{
  "protocol": "rein-extension/0.1",
  "type": "invoke",
  "sessionId": "session-示意",
  "requestId": "request-示意",
  "taskId": "task-进程-序号-时间",
  "callId": "call-1",
  "tool": "read_file",
  "path": "fixtures/hybrid-marker.txt",
  "ruleId": "read-file-content-v1",
  "targetVersion": "示意SHA256"
}
```

这里的 ID 和哈希是字段形状示意，实际运行会重新生成。模型返回的工具 ID 保留为 `callId`，其它 ID 由 core 绑定到这次运行。成功 terminal 包含 output 和完整 evidence：

```json
{
  "protocol": "rein-extension/0.1",
  "type": "terminal",
  "sessionId": "session-示意",
  "requestId": "request-示意",
  "taskId": "task-进程-序号-时间",
  "callId": "call-1",
  "status": "succeeded",
  "output": "文件中的真实 UTF-8 内容",
  "evidence": {
    "taskId": "task-进程-序号-时间",
    "callId": "call-1",
    "path": "fixtures/hybrid-marker.txt",
    "ruleId": "read-file-content-v1",
    "targetVersion": "实际文件SHA256"
  }
}
```

顺序是 `ready → invoke → started → terminal → EOF → exit 0`。core 会检查完整身份、工具规则、canonical 路径、当前 SHA-256 和 UTF-8 内容。成功只能有 output+evidence；`tool_failed` 只能有 error；`cancelled` 不带 output、error 或 evidence。terminal 后多余数据、非干净 EOF、非零退出或未回收，都会成为 `outcome_unknown`。

## 4、运行四个故障场景

每个故障只触发一次模型调用，答案为空，命令约定以 exit 1 结束。

错误身份：

```bash
cargo run --quiet --locked --manifest-path rust/Cargo.toml --example hybrid_stdio -- invalid
```

预期：`status=failed`、`reason=outcome_unknown`、`modelCalls=1`、`answer=null`。故障 host 只改变身份，output 仍来自真实临时 marker。

取消：

```bash
cargo run --quiet --locked --manifest-path rust/Cargo.toml --example hybrid_stdio -- cancel
```

预期：`failed/cancelled/1/null`。只有完整绑定的 cancel、匹配 cancelled terminal、干净 EOF、exit 0 和回收都成立时才确认取消。

崩溃：

```bash
cargo run --quiet --locked --manifest-path rust/Cargo.toml --example hybrid_stdio -- crash
```

预期：`failed/outcome_unknown/1/null`。invoke 可能已经送达，core 不能自动重试。

产生副作用后断连：

```bash
cargo run --quiet --locked --manifest-path rust/Cargo.toml --example hybrid_stdio -- disconnect-after-effect
```

预期：`failed/outcome_unknown/1/null`，并输出绝对 `ledgerPath`。复制该路径后检查：

```bash
wc -l "<输出中的绝对 ledgerPath>"
cat "<输出中的绝对 ledgerPath>"
```

文件应只有一行 request ID。这个 ledger 是故障 host 写入的临时副作用，不是 core 内存记录，也不是防重放器；它只说明本次运行没有自动重放，不提供 exactly-once 保证。

输出中的 `records` 是 core 的内存调用记录，用于审查本次进程生命周期；它不是持久 journal。

## 5、成功终态仍需要证据验收

core 采纳 output 前会重新 canonicalize 目标，比较当前 SHA-256、`targetVersion` 和 UTF-8 内容。目标在 host 读取期间被替换时，结果应是 unknown。本轮只验证“读取文件并把真实内容送回第二次离线模型调用”的闭环。通用领域任务的后置验收属于第 09 章计划实现的能力，当前没有现成的通用 post-validator。

Node 是受信任的首方进程，协议约束返回形状，但不等于操作系统沙箱。本轮没有 durable journal、批准 UI、恢复重放或 SDK 提炼。

## 6、练习

### 练习 06-1：改变输入

```bash
TMP_ROOT="$(mktemp -d)"
mkdir -p "$TMP_ROOT/fixtures"
printf 'marker: exercise-06-1\n' > "$TMP_ROOT/fixtures/hybrid-marker.txt"
REIN_HYBRID_ROOT="$TMP_ROOT" cargo run --quiet --locked --manifest-path rust/Cargo.toml --example hybrid_stdio -- normal
```

检查 `modelCalls` 为 2，且 `answer` 包含 `exercise-06-1`。

### 练习 06-2：比较取消与未知

```bash
cargo run --quiet --locked --manifest-path rust/Cargo.toml --example hybrid_stdio -- cancel
cargo run --quiet --locked --manifest-path rust/Cargo.toml --example hybrid_stdio -- crash
```

比较 JSON 的 `reason`、`modelCalls` 和 `answer`，说明为什么取消需要完整 ACK，而崩溃只能是 unknown。

### 练习 06-3：核对断连 ledger

运行断连命令，从 JSON 复制 `ledgerPath`，再用 `wc -l` 和 `cat` 检查一行 request ID。该结果说明本次测试没有自动重放，不能推出持久化恢复或 exactly-once。

## 7、回到第 07 章

现在回到 [07 上下文管理（Rust core）](/chapters/07-rust.html)。第 07 章把规则、目标、完整历史和发送上下文放进 Rust core 的预算边界；Node plugin 继续提供受控工具连接，但不会成为第二个 loop 或第二个权威状态机。


::: info 新版主题去向
本页保留原章节主题、内容和锚点。新版相关内容见[07 循环预算与重复动作](./loop-budget.md)、[08 取消、截止时间与资源生命周期](./cancellation.md)、[09 工具宿主与声明式扩展点](./tool-host.md)。新旧章号不是同一套编号；历史命令和快照保持原义。
:::

历史开放标注说明：本页冻结稿中的“限时免费”描述的是旧政策。当前本仓库自有代码与文档已按 Apache-2.0 永久开放，见 [内容开放说明](/access)。
