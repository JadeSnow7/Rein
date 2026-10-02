# 10 上下文结构：历史、状态与发送预算

**状态：新版初稿，未完成逐章衔接验收｜永久免费 · Apache-2.0**

本页正在迁移为新版主题入口。下面保留可检查的已有教学内容；其中数字命令与代码入口仍指历史主题，不能把运行当前工程等同于完成新版前驱快照验收。


第 06 章解决的是“什么时候停止”。但即使循环没有触发 `max_turns`，历史也可能长到模型无法接受。把旧消息随意删掉会丢失规则、目标或最近的依据；把它们全发出去又可能超过预算。本章把四个对象分开：规则是每次发送都要带上的约束，当前目标是本次运行要完成的请求，完整历史是审计用的事实，发送上下文是本轮真正交给模型的消息副本。

本文使用当前 workspace 中的 Rust core 实现。`chapter-snapshots/rein-ch07.tar.gz` 仅用于可选的历史复现；不要求先阅读材料 2，也不依赖 Git 标签。TS 工具和 SDK 仍通过扩展边界保留，但上下文状态与预算由 Rust core 负责。Rust 需要支持 2021 edition 的工具链和网络已缓存或可用的 Cargo 依赖；Node 命令只使用内置 `fs`。示例是确定性的离线适配器：它从发送给它的最新完整工具结果拼接答案，不代表真实模型能力，也不会现场读取 fixture 中路径为 `a`、`b` 的文件。

## 先准备共享案例

<a id="context-inputs"></a>

从书根复制公开案例到临时文件。这样可以改自己的输入，同时保留原始 fixture：

```bash
export REIN_CH07_CASE="$(mktemp -d)/ch07-context.json"
cp fixtures/cases/ch07-context.json "$REIN_CH07_CASE"
cargo run --quiet --manifest-path rust/Cargo.toml --example ch07_context -- "$REIN_CH07_CASE"
```

案例文件包含 `unit` 和多个 `cases`。每个 case 给出 `rules`、`goal`、完整 `history`、`budget` 和 `managed`。`history` 中的 assistant tool call 与 tool result 已经是输入事实；程序不会因为看到 `path: "a"` 就去现场打开文件。离线适配器检查它真正收到的消息，找到最新完整工具组的结果，再按声明 call 顺序用换行拼成答案。

输出最外层是 `unit: "estimated-bytes-v1"` 和 `cases`。其中 `unmanaged-overflow` 仍送出全部历史，适配器发现发送量超过 300 后返回 `model_error`；`managed-window` 在相同输入和预算下裁掉最旧组，保留规则、目标和最新工具组，得到 `final_answer`。`required-overflow` 的规则和目标本身放不下，因而没有模型请求。`multi-tool-boundary` 的旧工具组整体移除，最新组中的两个结果一起保留；`invalid-orphan` 与 `invalid-missing` 在发模型前以 `invalid_context_history` 结束。`unicode-content` 用中文和 emoji 检查 UTF-8 估算。

对 `managed-window`，同一输入的关键数字可以直接从 `context_prepared` 事件核对：

| 项目 | 值 | 含义 |
| --- | ---: | --- |
| `beforeUnits` | 644 | 规则、完整历史和当前目标全部发送前的估算量 |
| `requiredUnits` | 239 | 规则、当前目标组 `g4` 与最新工具组 `g1` 的必要内容 |
| `afterUnits` | 239 | 移除旧组后真正发送的估算量 |
| `removedGroups` | `g0` | 最旧的非必要组 |
| `keptGroups` | `g1`, `g4` | 最新完整工具组和当前目标组 |

`requiredUnits` 能放进预算时才允许裁剪；如果规则、目标或最新工具组本身超过预算，循环应停止而不是删除依据。

## 估算单位不是 token

<a id="budget-estimate"></a>

`estimated-bytes-v1` 是本地可复算的教学单位。它使用字符串的 UTF-8 字节长度，但把 JSON 数字固定估作 8；它不是 JSON 在线字节数，也不是服务 token。消息估算的实际函数是：

```rust
fn estimated_message(message: &Message) -> usize {
    8 + message.role.as_bytes().len()
        + message.content.as_bytes().len()
        + message.tool_call_id.as_ref().map_or(0, |id| id.as_bytes().len())
        + message.tool_calls.iter().map(|call| {
            8 + call.id.as_bytes().len()
                + call.name.as_bytes().len()
                + estimated_json(&call.arguments)
        }).sum::<usize>()
}
pub fn estimated_units(messages: &[Message]) -> usize {
    messages.iter().map(estimated_message).sum()
}
```

`estimated_json` 对 null、布尔、数字、字符串、数组和对象递归计数，并为分隔符加固定项。上下文事件的 `beforeUnits` 与 `afterUnits` 还包括注入的 system 规则，但不包括工具 schema、HTTP 包络或服务端实际计费。读者可以用案例的 `unit`、消息文本和规则复算结果，却不能把它换算成某个模型的 token 余量。

## 完整历史与发送副本

<a id="atomic-history"></a>

Rust 用拥有型 `Vec<Message>` 保存审计历史。`ContextConfig` 的 `history` 先复制，再追加当前 goal；每轮 `prepare_context` 从这份完整历史重新分组。一个有 tool call 的 assistant 消息和它后面全部对应的 tool result 是一个组，组 ID 按完整历史起点命名为 `g0`、`g1`……多调用组不能拆开。规则另生成 system 消息，不会被旧历史覆盖。

```rust
let context = options.context.clone();
let history_len = context.as_ref().map(|c| c.history.len()).unwrap_or(0);
let mut messages = context.as_ref().map(|c| c.history.clone()).unwrap_or_default();
messages.push(Message { role: "user".into(), content: prompt.into(), tool_call_id: None, tool_calls: vec![] });
```

`prepare_context` 只返回本轮发送的 `Vec<Message>`，然后将同一独立副本放进 `ModelRequested` 并传给 adapter；`LoopResult.messages` 仍是没有裁剪的完整审计历史。Rust 的 clone 和所有权让发送副本的修改不会回写审计数组。`Option` 表示 context 配置可能不存在，`Result` 则把非法历史和预算拒绝变成必须处理的路径。

配置是在调用 `run_agent_loop_with_options` 时接入的；下面是调用点的节选，不是独立可编译程序：

```rust
run_agent_loop_with_options(
    &adapter,
    &workspace,
    &case.goal,
    LoopOptions {
        max_turns: 2,
        context: Some(ContextConfig {
            rules: case.rules,
            history: case.history,
            budget: case.budget,
            manage: case.managed,
        }),
        ..LoopOptions::default()
    },
).await;
```

四个字段分别进入规则、审计历史、单位预算和 managed/unmanaged 选择；真正的 adapter 仍由 loop 负责调用。

## managed 与 unmanaged 的对照

<a id="managed-run"></a>

```bash
cargo run --quiet --manifest-path rust/Cargo.toml --example ch07_context -- "$REIN_CH07_CASE"
```

`managed: false` 不是关闭校验，而是只保留完整历史并记录 `context_prepared`；它不裁剪、不提前预算拒绝，所以离线 adapter 仍会因超过 300 返回 `model_error`。`managed: true` 先验证历史，计算必要组，再从最旧的非必要组开始整体删除，直到发送量不超过预算。当前目标组和最新工具组始终必要；它们本身超过预算时直接 `context_budget_exhausted`，请求数为 0。

本章的真实多轮上下文回归位于 `rust/tests/loop.rs` 的 `ch07_context_tracks_latest_group_across_real_tool_rounds` 与 `ch07_context_stops_when_new_latest_group_does_not_fit`：前者覆盖两轮真实 `read` 结果形成新工具组并替换上一轮的最新必需组，后者覆盖新组成为最新依据后因预算不够而停止。`ch05_loop.rs` 和 `ch06_loop.rs` 仍证明旧入口兼容，但不替代这两个 ch07 loop 测试；本章 fixture 的 `a`、`b` 仍是 seeded 历史，不是现场文件读取。

运行结果中的 `sentMessages` 记录真正进入 adapter 的消息二维数组；`events` 里的 `context_prepared` 记录 `beforeUnits`、`afterUnits`、`requiredUnits`、`keptGroups` 和 `removedGroups`。这能检验管理发生在 loop 内，而不是只测一个未被调用的裁剪函数。

## 源码中的顺序和失败边界

`run_agent_loop_with_options` 先验证 `ContextConfig.budget` 和历史结构；非法角色、孤立 result、缺失 result、重复 call ID 或不相邻 result 都在第一次模型请求前返回 `invalid_context_history`。随后 `prepare_context` 计算规则、全部组与必要组：

```rust
let required_units = groups.iter()
    .filter(|group| required.contains(&group.id))
    .flat_map(|group| messages[group.start..group.end].iter())
    .map(estimated_message).sum::<usize>() + rules_units;
if required_units > config.budget {
    return Err(StopReason::ContextBudgetExhausted);
}
```

上面是实际判断核心；真实实现会在返回错误前写入完整的 `ContextRejected` 事件，事件字段包括 `turn`、`unit`、`mode`、`budget`、`beforeUnits` 和 `requiredUnits`。通过后才删除非必要旧组，并重新按原顺序构造发送数组。`context_rejected` 与最终 `stopped` 分开记录：前者说明哪条上下文规则拒绝了请求，后者说明循环停止原因。非法配置的 errorCode 是 `invalid_context_config`，但停止原因仍是 `invalid_context_history`。

Rust 的 `match`、`Option` 和 `Result` 提供编译期和控制流上的帮助，不能替代 JSON 解码、有限数字检查或历史关联检查。`serde` 的 `toolCallId` 映射只解决字段名，不证明结果属于正确的 assistant call。

## 失败实验

<a id="context-failure"></a>

为了只观察失败类型，可以在输出中筛选三个 case：

```bash
cargo run --quiet --manifest-path rust/Cargo.toml --example ch07_context -- "$REIN_CH07_CASE" > /tmp/rein-ch07-rust.json
node -e 'const x=require("/tmp/rein-ch07-rust.json"); for(const c of x.cases) if(c.result.reason!=="final_answer") console.log(c.id,c.result.reason,c.requests)'
```

预期 `unmanaged-overflow` 是 `model_error/1`，`required-overflow` 是 `context_budget_exhausted/0`，两个 invalid case 是 `invalid_context_history/0`。不要因为所有 CLI 命令退出 0 就认为这些业务 case 成功；必须读取 JSON 中的 `reason`、`requests` 和发送消息。

## 练习

<a id="practice-07-1"></a>

### 练习 07-1：改变最新事实

每项练习都从原始 fixture 复制独立输入。先做 07-1：

```bash
export REIN_CH07_P1="$(mktemp -d)/case.json"
cp fixtures/cases/ch07-context.json "$REIN_CH07_P1"
node -e 'const fs=require("fs"); const p=process.argv[1]; const x=JSON.parse(fs.readFileSync(p)); const c=x.cases.find(c=>c.id==="managed-window"); const m=c.history.find(m=>m.role==="tool" && m.toolCallId==="latest-a"); m.content="最新事实 A（已更正）"; fs.writeFileSync(p,JSON.stringify(x));' "$REIN_CH07_P1"
cargo run --quiet --manifest-path rust/Cargo.toml --example ch07_context -- "$REIN_CH07_P1"
```

只应看到 `managed-window` 的答案变为 `最新事实 A（已更正）\n最新事实 B`；检查 `sentMessages` 中仍有完整最新工具组，且审计 `result.messages` 没有被物理裁剪。不要只改 `expected.answer`，因为 adapter 不读取 expected。

<a id="practice-07-2"></a>

### 练习 07-2：预算边界 239/238

用两个从原始 fixture 复制的单独文件，避免 07-1 的内容改动影响必要单位：

```bash
export REIN_CH07_P2_239="$(mktemp -d)/case.json"
export REIN_CH07_P2_238="$(mktemp -d)/case.json"
cp fixtures/cases/ch07-context.json "$REIN_CH07_P2_239"
cp fixtures/cases/ch07-context.json "$REIN_CH07_P2_238"
node -e 'const fs=require("fs"); const p=process.argv[1]; const x=JSON.parse(fs.readFileSync(p)); x.cases=x.cases.filter(c=>c.id==="managed-window"); x.cases[0].budget=239; fs.writeFileSync(p,JSON.stringify(x));' "$REIN_CH07_P2_239"
node -e 'const fs=require("fs"); const p=process.argv[1]; const x=JSON.parse(fs.readFileSync(p)); x.cases=x.cases.filter(c=>c.id==="managed-window"); x.cases[0].budget=238; fs.writeFileSync(p,JSON.stringify(x));' "$REIN_CH07_P2_238"
cargo run --quiet --manifest-path rust/Cargo.toml --example ch07_context -- "$REIN_CH07_P2_239"
cargo run --quiet --manifest-path rust/Cargo.toml --example ch07_context -- "$REIN_CH07_P2_238"
```

239 应为 `final_answer`、1 次请求；238 应为 `context_budget_exhausted`、0 次请求。观察事件中的 `requiredUnits` 与 `budget`，不要删除规则或最新工具组来“挤出”答案。

<a id="practice-07-3"></a>

### 练习 07-3：删除一个对应结果

从原始 fixture 复制第三个独立文件，删除 managed case 的最后一个 tool result，保留其 assistant call：

```bash
export REIN_CH07_P3="$(mktemp -d)/case.json"
cp fixtures/cases/ch07-context.json "$REIN_CH07_P3"
node -e 'const fs=require("fs"); const p=process.argv[1]; const x=JSON.parse(fs.readFileSync(p)); const c=x.cases.find(c=>c.id==="managed-window"); c.history.pop(); fs.writeFileSync(p,JSON.stringify(x));' "$REIN_CH07_P3"
cargo run --quiet --manifest-path rust/Cargo.toml --example ch07_context -- "$REIN_CH07_P3"
```

预期 `invalid_context_history`、请求数 0，并在 `context_rejected.errorCode` 中看到 `invalid_context_history`；最终 `LoopResult.error` 也保存这个短码。修复配对后再运行，才能回到正常管理路径；这一步检查的是历史完整性，不是工具文件是否存在。

本章的 Rust 入口只覆盖当前快照的上下文管理；后续演进可能增加策略和停止原因，但不会改变本节对完整历史、发送副本和预算边界的区分。


## 提示词示例

```text
请区分完整历史与本轮发送副本，保留规则、当前目标和完整调用结果组。
裁剪不能拆开多工具调用与结果；必要内容超预算时停止。
比较发送前后单位，并说明估算不等于服务token。
```

“副本”保护历史事实；“完整组”约束裁剪；“不等于token”限制测量解释。**试用状态：未试用。** 使用前请阅读[《提示词示例使用说明》](../prompt-examples.md)。
