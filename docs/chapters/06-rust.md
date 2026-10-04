---
prev: { text: 05 核心 Agent Loop（Rust core）, link: /chapters/05-rust.html }
next: { text: 06 插件附页, link: /chapters/06-plugin.html }
---

# 06 循环控制（Rust）

上一章的 Rust 循环已经把搜索和读取结果作为下一轮消息。现在的问题是生命周期：一个 Agent 继续产生工具调用时，谁决定它必须停下？本章在 Tokio 异步循环中分别控制模型回合、工具派发、重复动作、取消和整体截止时间。你会看到 Rust 的 future、`select!` 和原子信号怎样共同表达这个边界。

正文对应携带的 `chapter-snapshots/rein-ch06.tar.gz` 文件树快照；它不是 Git 提交或标签。快照是可选的历史复现材料；主线直接在当前 workspace 准备输入，不要求先阅读材料 2。Rust core 负责循环状态与停止边界，TS 工具仍可通过插件附页接入：

```bash
export REIN_CH06_DIR="$(mktemp -d)"
printf 'marker: ch06 alpha\n' > "$REIN_CH06_DIR/README.md"
printf 'marker: ch06 beta\n' > "$REIN_CH06_DIR/guide.md"
cargo build --manifest-path rust/Cargo.toml
```

## 先看正常的三回合

<a id="loop-entry"></a>

```bash
cargo run --manifest-path rust/Cargo.toml --example ch06_loop -- normal "$REIN_CH06_DIR"
```

回放适配器第一次返回搜索调用 `search-1`，循环先执行 `search_files`；第二次模型看到搜索结果后返回 `read-1`、`read-2`，循环才执行两个 `read_file`；第三次以真实内容组成 `完成：marker: ch06 alpha | marker: ch06 beta`。JSON 中 `requests` 和 `settledRequests` 都是 3，说明示例等待结束后没有额外的迟到模型回合。把 README 改成另一个 marker 后重跑，最终答案随输入变化；这是工具消息推动下一轮的证据，而不是脚本直接打印固定答案。

## 两种预算在不同边界生效

<a id="budget-and-stop"></a>

```bash
cargo run --manifest-path rust/Cargo.toml --example ch06_loop -- budget "$REIN_CH06_DIR"
cargo run --manifest-path rust/Cargo.toml --example ch06_loop -- tool-zero "$REIN_CH06_DIR"
cargo run --manifest-path rust/Cargo.toml --example ch06_loop -- zero "$REIN_CH06_DIR"
```

`LoopOptions.max_turns` 统计进入模型适配器的回合，`max_tool_calls` 统计真正调用 `dispatch_readonly` 的次数。`budget` 的一次模型回合返回 `one`、`two`：`one` 产生 `ToolResult`，`two` 产生 `ActionSkipped { reason: ToolBudgetExhausted }`。`tool-zero` 的一次模型请求中两个调用都跳过。`zero` 的 `max_turns` 为 0，因此模型请求数为 0，停止原因为 `max_turns`。Rust 程序以退出码 0 输出这些业务失败状态，只说明 CLI 成功完成报告；不能把它当成任务成功。

每个真实工具结果都同时有 `tool_result` 事件和带 `toolCallId` 的 `tool` 消息。跳过动作没有工具结果，也没有未执行调用的消息，因此下一轮看不到虚构的输出。事件中的停止原因使用 snake_case；`toolCallId` 保持跨语言合同的 camelCase。

## 重复动作的身份不包含调用 ID

<a id="duplicate-action"></a>

```bash
cargo run --manifest-path rust/Cargo.toml --example ch06_loop -- duplicate "$REIN_CH06_DIR"
```

Rust 回放中的两个调用 ID 固定为 `duplicate-1` 与 `duplicate-2`。两次调用的动作都是 `read_file`，参数都是 README；`duplicate_limit: 1` 因而允许第一次，第二次被 `ActionSkipped`，最终 `DuplicateAction`。循环通过工具名和 `canonical_json` 拼接身份，`canonical_json` 对对象键递归排序，数组顺序不改变。调用 ID 仍用于把事件与消息对应起来，但不能让同一个语义动作通过改 ID 逃过阈值。

`duplicate_limit` 是允许同一身份实际派发的次数。它不等于模型可以返回多少次调用，也不负责判断读取内容是否正确。若参数中有数组，不能为了去重擅自排序，因为那会改变参数语义。

## 空搜索与模型错误

另建一个没有 marker 的目录：

```bash
export REIN_CH06_EMPTY="$(mktemp -d)"
printf 'unrelated\n' > "$REIN_CH06_EMPTY/other.txt"
cargo run --manifest-path rust/Cargo.toml --example ch06_loop -- normal "$REIN_CH06_EMPTY"
```

搜索结果为空时，回放适配器返回 `no_matches` 错误；循环报告 `state: "failed"`、`reason: "model_error"`，没有 `answer`。Rust CLI 退出 0 只表示它成功打印了这个业务结果。真正适配器的解析或传输异常也应落到 `model_error`，而空的最终文本则是另一个 `empty_final` 原因。

## 取消、future 和资源生命周期

<a id="cancellation"></a>

```bash
cargo run --manifest-path rust/Cargo.toml --example ch06_loop -- cancel-before "$REIN_CH06_DIR"
cargo run --manifest-path rust/Cargo.toml --example ch06_loop -- cancel "$REIN_CH06_DIR"
cargo run --manifest-path rust/Cargo.toml --example ch06_loop -- cancel-at-return "$REIN_CH06_DIR"
cargo run --manifest-path rust/Cargo.toml --example ch06_loop -- cancel-between-tools "$REIN_CH06_DIR"
```

`cancel-before` 在第一次模型请求前检查 `ControlSignal`，所以 `requests: 0`。`cancel` 的回放 future 等待 100ms，而另一个 Tokio 任务在 20ms 调用 `ControlSignal::cancel`；`await_control` 用 `tokio::select!` 观察 future 与取消/截止时间，返回 `cancelled`，迟到 future 被丢弃。`cancel-at-return` 在返回“迟到答案”之前设置标志，循环在收到结果后再次检查，因此答案不会进入 `LoopResult`。这只隔离本地结果；它没有撤回服务器已处理的网络请求。

`cancel-between-tools` 安装 `ToolObserver`，第一项 `one` 完成后 observer 调用 `cancel`。第一项的 `ToolResult` 和消息被保留，第二项 `two` 记录为 `ActionSkipped`。observer 是调用方提供的普通闭包，本例只在观察点调用 `cancel`；它不是权限隔离或沙箱。Rust 的 `Arc<AtomicBool>` 让 signal 可克隆并在线程间观察，`Release/Acquire` 保证取消状态可见。

<a id="deadline"></a>

整体截止时间覆盖模型等待和同轮工具间隔：

```bash
cargo run --manifest-path rust/Cargo.toml --example ch06_loop -- timeout "$REIN_CH06_DIR"
```

示例传入 30ms 的 `timeout`，而模型 future 等待 100ms，结果为 `timeout`。这与模型适配器自己的 HTTP 请求超时属于不同层次；本地睡眠只是在可控条件下制造竞争，不能当作服务性能测量。

## 走读 Rust 核心

`LoopOptions` 把预算、signal、timeout 和 observer 组合为一次运行的控制面；`ControlSignal` 是 `Arc<AtomicBool>` 的轻量句柄：

```rust
#[derive(Clone, Debug, Default)]
pub struct ControlSignal(Arc<AtomicBool>);
pub struct LoopOptions {
    pub max_turns: usize,
    pub max_tool_calls: usize,
    pub duplicate_limit: usize,
    pub timeout: Option<Duration>,
    pub signal: Option<ControlSignal>,
    pub tool_observer: Option<ToolObserver>,
}
```

接口中的 `ModelAdapter::complete_controlled` 接收 signal，但当前 OpenAI 适配器实现仍调用普通 `complete`，没有把 `ControlSignal` 传入 HTTP 层；本例的取消依靠 `await_control` 用 `tokio::select!` 丢弃等待中的模型 future，并在返回前后检查 signal。未选中的 future 被 drop，但这不保证远端服务撤销已处理的请求。

模型响应先产生 `ModelReceived`、再进入消息历史。工具循环随后按顺序检查预算、控制信号和重复身份；只有检查通过才递增 `tool_started`、同步调用 `dispatch_readonly`，写入 `ToolResult` 与工具消息，再调用 observer：

```rust
let result = dispatch_readonly(&call, workspace);
let content = if result.ok {
    result.output.clone().unwrap_or_default()
} else {
    serde_json::json!({"ok":false,"error":result.error}).to_string()
};
events.push(LoopEvent::ToolResult {
    turn,
    call: call.clone(),
    tool_call_id: call.id.clone(),
    result: result.clone(),
});
messages.push(Message {
    role: "tool".into(),
    content,
    tool_call_id: Some(result.tool_call_id.clone()),
    tool_calls: vec![],
});
if let Some(observer) = &options.tool_observer {
    observer(&call, &result, &signal);
}
```

observer 是调用方提供的任意 trusted hook；本例只用它在结果后取消，它本身不是权限隔离或沙箱。剩余调用仅写 `ActionSkipped`。旧的 `run_agent_loop_with_adapter(..., max_turns)` 保留默认选项，因而 ch05 的公共调用仍兼容。

`Pin<Box<dyn Future<...> + Send>>` 允许不同适配器提供异步计算，`select!` 未选中的分支在取消时释放 future。若适配器自己启动外部进程，它仍必须在自己的 future 取消路径清理进程；本章离线回放没有子进程，不能据此声称外部资源清理已被全面验证。工具 schema 仍只是模型可见声明，真实路径限制在 `dispatch_readonly`。

## 练习

<a id="practice-06-1"></a>

### 练习 06-1：改变重复阈值

在快照的 `rust/examples/ch06_loop.rs` 中将 `duplicate_limit` 的 `1` 改为 `2`，运行 `duplicate`。预期第一次和第二次均有 `ToolResult`，第三次才有 `ActionSkipped` 与 `duplicate_action`；Rust 示例的调用 ID 是固定的 `duplicate-1`、`duplicate-2`，不要假设它与 TypeScript 的动态编号相同。恢复为 1 后只应启动一次工具。`max_turns` 为 3，不会先挡住该实验。

<a id="practice-06-2"></a>

### 练习 06-2：改变取消发生点

保持示例模型 100ms 延迟，分别运行四种模式：

```bash
for mode in cancel-before cancel cancel-at-return cancel-between-tools; do
  cargo run --manifest-path rust/Cargo.toml --example ch06_loop -- "$mode" "$REIN_CH06_DIR"
done
```

实际计数应分别是：`cancel-before` 为 0 次模型/0 次工具，`cancel` 为 1/0，`cancel-at-return` 为 1/0，`cancel-between-tools` 为 1/1（第二个工具跳过）。每次检查 `ModelRequested`、`ModelReceived`、`ToolResult`、`ActionSkipped` 和 `settledRequests`，不要用随机竞态推断结论。

<a id="practice-06-3"></a>

### 练习 06-3：同轮工具预算

在 `LoopOptions` 初始化处把 `budget` 的 `max_tool_calls` 从 1 改为 2，再运行 `budget`。第一次模型回合的两个调用都有真实结果；该回放下一回合仍返回两个调用，累计预算已满，因此第二次模型请求后两个调用都进入 `ActionSkipped`，预期 `requests: 2`、两个 `ToolResult` 和两个跳过事件。改回 1 后第二个只能在第一次回合出现在 `ActionSkipped` 中，`messages` 不应出现它的 `toolCallId`。提高工具预算只扩大派发边界，不保证下一轮一定得到最终答案。

本文以携带的 `chapter-snapshots/rein-ch06.tar.gz` 为阅读时点。


::: info 新版主题去向
本页保留原章节主题、内容和锚点。新版相关内容见[07 循环预算与重复动作](./loop-budget.md)、[08 取消、截止时间与资源生命周期](./cancellation.md)、[09 工具宿主与声明式扩展点](./tool-host.md)。新旧章号不是同一套编号；历史命令和快照保持原义。
:::
