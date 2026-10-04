---
prev: { text: 04 消息与工具, link: /chapters/04.html }
next: { text: 06 循环控制（Rust core）, link: /chapters/06-rust.html }
---

# 05 Rust：用 Result 把工具结果送回模型

本章把循环控制迁入 Rust core：Rust 负责状态、回合和工具结果，TS 保留为可复用的工具与 SDK 生态扩展。本章用 Rust 写同一个只读循环：搜索 `marker:`，读取搜索结果中的两个文件，再把内容交给下一轮。`rust/src/rein/mod.rs` 提供合同数据结构、只读工具与 OpenAI 兼容非流式 HTTP 接口，`rust/src/rein/loop.rs` 提供循环，`rust/examples/ch05_loop.rs` 提供离线适配器。`mod.rs` 的 replay 可接收 Anthropic 形状 JSON，经 `parse_anthropic_turn` 转成 `ModelTurn`，再由 `ModelAdapter` 包装接入 loop；这是离线格式适配，没有 Anthropic 真实网络入口。本章主演示仍是消费工具消息的动态离线 adapter。适配器实现 `ModelAdapter`，再传给 `run_agent_loop_with_adapter`；这些是本章附带的最小前置，不表示前四章已经完整交付。

<span id="loop-entry"></span><span id="search-read"></span>

## 运行准备

所有命令从仓库根目录运行。新读者先确认工具链并首次构建；首次构建不使用 `--offline`，让 Cargo 按锁定依赖获取尚未缓存的包：

```bash
rustc --version
cargo --version
cargo build --manifest-path rust/Cargo.toml
```

然后建立本章唯一的输入目录，目录中只有两份文件：

```bash
export REIN_CH05_DIR="$(mktemp -d)"
printf 'marker: README alpha\n' > "$REIN_CH05_DIR/README.md"
printf 'marker: NOTES beta\n' > "$REIN_CH05_DIR/notes.md"
```

```bash
cargo fmt --manifest-path rust/Cargo.toml -- --check
cargo run --manifest-path rust/Cargo.toml --example ch05_loop -- workspace "$REIN_CH05_DIR" multi
cargo run --manifest-path rust/Cargo.toml --example ch05_loop -- workspace "$REIN_CH05_DIR" single
```

后续命令可继续省略 `--offline`。默认 `multi` 是三次请求：第一轮 `search-1`，第二轮两个 read，第三轮最终回答；预期答案是 `多文件摘要：marker: README alpha | marker: NOTES beta`。`single` 读取 README 后回答。

## Rust 如何承载循环

`rust/src/rein/mod.rs` 的数据结构是可序列化的拥有型数据：`Message` 持有 `String` 和 `Vec<ToolCall>`，`ToolCall` 持有 JSON `Value`，`ToolResult` 以 `tool_call_id` 配对结果。`serde(rename = "toolCallId")` 把 Rust 字段映射到共享 wire 名称。

`rust/src/rein/loop.rs` 把模型接入抽象成 trait：

```rust
pub trait ModelAdapter {
    fn complete<'a>(&'a self, messages: &'a [Message], tools: &'a [ToolDefinition])
      -> Pin<Box<dyn Future<Output = Result<ModelTurn, ToolError>> + Send + 'a>>;
}
```

`&[Message]` 是本轮借用的只读视图；返回 `ModelTurn` 后循环取得拥有权追加到 `Vec<Message>`。`Result` 强迫调用者处理模型错误。`Pin<Box<dyn Future + Send>>` 把可能跨 await 的异步结果放在稳定堆地址，并允许在线程间传递。枚举经 `serde(rename_all = "snake_case")` 后，停止原因 wire 值为 `final_answer`、`empty_final`、`model_error`、`max_turns`。

回放适配器的 `multi` 方法内分成不同轮的分支：第一轮没有 tool 消息时生成 search；第二轮只在已有搜索结果时用实际路径生成 read calls；第三轮才从已有 read 消息拼答案。第二轮的核心摘录如下：

```rust
let paths: Vec<&str> = tool_messages
    .first()
    .map(|s| s.lines().take(2).collect())
    .unwrap_or_default();
let calls: Vec<ToolCall> = paths.iter().enumerate().map(|(i, path)| ToolCall {
    id: format!("read-{}", i + 1),
    name: "read_file".into(),
    arguments: serde_json::json!({"path": path}),
}).collect();
```

循环先建 user 消息，每轮克隆快照后调用 adapter；模型消息追加，若无 tool call 就在非空文本时 `FinalAnswer`，空白时 `EmptyFinal`。有调用则按顺序 dispatch：

```rust
for call in response.tool_calls {
    let result = dispatch_readonly(&call, workspace);
    events.push(LoopEvent::ToolResult { turn, call: call.clone(),
        tool_call_id: call.id.clone(), result: result.clone() });
    let content = if result.ok { result.output.unwrap_or_default() }
        else { serde_json::json!({"ok":false,"error":result.error}).to_string() };
    messages.push(Message { role: "tool".into(), content,
        tool_call_id: Some(result.tool_call_id), tool_calls: vec![] });
}
```

第三次请求中，`search-1` 的输出已经在第二轮决定两个 `read_file` 调用；两个 tool 消息按 ID 配对，不能只按数组位置猜测。下面只展示消息序列中的相关字段，省略其他字段，不是完整的顶层 JSON：

```json
{"role":"tool","toolCallId":"search-1","content":"README.md\nnotes.md"}
{"role":"assistant","toolCalls":[{"id":"read-1","name":"read_file","arguments":{"path":"README.md"}},{"id":"read-2","name":"read_file","arguments":{"path":"notes.md"}}]}
{"role":"tool","toolCallId":"read-1","content":"marker: README alpha\n"}
{"role":"tool","toolCallId":"read-2","content":"marker: NOTES beta\n"}
```

工具 schema 与 TS 同样声明 `path` 或 `needle` 为字符串且禁止额外字段；这只是声明，不等于完整的运行时 JSON Schema 校验。事件包含完整消息、工具定义、模型消息和结果，是本次运行的观察轨迹，不是跨进程恢复。默认示例 32 轮，live 入口 4 轮且不自动重试。

## 失败实验、live 与练习

以下命令继续使用 `$REIN_CH05_DIR`，从仓库根逐条运行：

```bash
cargo run --manifest-path rust/Cargo.toml --example ch05_loop -- workspace "$REIN_CH05_DIR" recovery
cargo run --manifest-path rust/Cargo.toml --example ch05_loop -- workspace "$REIN_CH05_DIR" empty
cargo run --manifest-path rust/Cargo.toml --example ch05_loop -- workspace "$REIN_CH05_DIR" exhausted
cargo run --manifest-path rust/Cargo.toml --example ch05_loop -- workspace "$REIN_CH05_DIR" limit
```

预期依次为 `completed/final_answer`（先 `path_invalid` 后读 README）、`failed/empty_final`、`failed/model_error`、`failed/max_turns`；每次保留 `stopped` 事件，`exhausted` 的错误消息为 `replay exhausted`。

真实服务显式开启。示意值需要替换为个人配置；只有显式使用 `--live` 才会发出网络请求，本章离线跟做不调用服务：

```bash
export REIN_BASE_URL="https://个人服务.example/v1" REIN_API_KEY="个人密钥" REIN_MODEL="个人模型"
cargo run --manifest-path rust/Cargo.toml --example ch05_loop -- --live workspace "$REIN_CH05_DIR" multi "请搜索 marker: 并读取命中文件"
```

程序不自动加载 `.env`，workspace 仍为同一个只含 README/notes 的 `$REIN_CH05_DIR`。live 只有 `completed/final_answer` 以退出码 0 结束，其余 `failed` 结果以退出码 1 结束；离线失败演示仍打印 JSON 并以退出码 0 结束。

<span id="practice-05-1"></span><span id="practice-05-2"></span><span id="practice-05-3"></span>

**05-1：多文件读取。** 运行 `cargo run --manifest-path rust/Cargo.toml --example ch05_loop -- workspace "$REIN_CH05_DIR" multi`。验收第三次模型输入含两个真实 tool 结果，答案为 `多文件摘要：marker: README alpha | marker: NOTES beta`。

**05-2：失败后恢复。** 运行 `cargo run --manifest-path rust/Cargo.toml --example ch05_loop -- workspace "$REIN_CH05_DIR" recovery`。验收 `missing-1` 的 `path_invalid`、`ok-1` 的真实读取和最终 `final_answer`。

**05-3：停止边界。** 运行下面两条命令：

```bash
cargo run --manifest-path rust/Cargo.toml --example ch05_loop -- workspace "$REIN_CH05_DIR" empty
cargo run --manifest-path rust/Cargo.toml --example ch05_loop -- workspace "$REIN_CH05_DIR" limit
```

验收 JSON 原因精确为 `empty_final`、`max_turns`，末尾事件 `stopped`，进程不 panic。


::: info 历史主题与现行路线
本页保留旧数字 05 的 Rust 循环实现与验收范围；[旧五篇 Rust 迁移章](./rust-migration.md)和[旧循环章](./agent-loop.md)继续作为对照。现行六部分的 05–06 见[第二部分规划](../roadmap/part-02.md)，历史命令和快照保持原义。
:::
