# 08 取消、截止时间与资源生命周期

**状态：旧五篇历史正文，未作为现行 08 验收｜永久免费 · Apache-2.0**

预算只能限制数量，不能保证一次等待会结束。模型请求正在等待，用户取消了任务，此时要停止采纳迟到结果，并释放本地持有的资源。下一章会把这项责任延伸到宿主进程。

所有命令从仓库根目录运行，使用自己的临时工作区：

```bash
export REIN_CH06_DIR="$(mktemp -d)"
printf 'marker: ch06 alpha\n' > "$REIN_CH06_DIR/README.md"
printf 'marker: ch06 beta\n' > "$REIN_CH06_DIR/guide.md"
```

当前命令仍使用旧 `ch06_loop` 示例名，表示已有执行入口；新版独立快照衔接尚未验收。

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

## 释放Future意味着什么

`Future`表达尚未完成的计算，`await`等待其结果。取消选择了另一个分支时，原Future会被丢弃，本地借用随之结束。但若请求已到达外部服务，释放本地Future不能证明远端停止。启动子进程的执行器还必须显式处理终止与回收；只取消等待可能留下仍在运行的进程。

取消开始前未派发、已知取消完成、结果未知要分别记录。默认关闭自动重试；后续17才在明确失败条件下讨论有限修复。只读操作的取消测试不能证明写入具有幂等性。

本章验收四个时机：请求前、等待中、返回时和两次工具之间。检查实际请求数、已采纳结果及未执行事件，不以打印“cancelled”作为唯一判断。

## 提示词示例

```text
请在当前循环中验证请求前、等待中、返回时和两次工具之间的取消。
保留已完成动作的真实结果，停止尚未派发的调用，并检查迟到答案没有被采纳。
说明本地Future释放能证明什么，不能证明什么。
```

“四个时机”定位竞争边界；“已完成”与“尚未派发”区分已有事实；“不能证明什么”要求解释远端副作用的限制。**试用状态：未试用。** 使用前请阅读[《提示词示例使用说明》](../prompt-examples.md)。
