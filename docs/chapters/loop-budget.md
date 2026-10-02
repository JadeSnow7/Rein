# 07 循环预算与重复动作

**状态：新版初稿，逐章衔接待验收｜永久免费 · Apache-2.0**

循环能够连续读取资料，也可能一直重复读取。我们需要分别回答：已经请求模型多少次，真正执行了多少个工具，同一个动作重复了多少次？模型一次返回两个调用，不等于宿主已经执行两次。

所有命令从仓库根目录运行，使用自己的临时工作区：

```bash
export REIN_CH06_DIR="$(mktemp -d)"
printf 'marker: ch06 alpha\n' > "$REIN_CH06_DIR/README.md"
printf 'marker: ch06 beta\n' > "$REIN_CH06_DIR/guide.md"
```

当前命令仍使用旧 `ch06_loop` 示例名，表示已有执行入口；新版独立快照衔接尚未验收。

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

## 一组调用没有完成时

预算阻止了第二项调用，历史就不再是完整的调用结果组。保留模型原始请求和已执行结果用于审查，记录剩余调用未执行，然后停止本次循环。不能伪造第二项输出，也不能直接把残缺关系发给下一轮模型。

练习：把工具预算从1改为2，观察实际派发和跳过事件变化；再把模型预算设为0，确认适配器完全没有被调用。次数增加不代表任务已成功，最终答案仍需符合02建立的验收问题。

## 提示词示例

```text
请分别统计模型请求次数、模型返回调用数和实际工具派发数。
预算不足时逐条记录未执行调用并停止本轮流程，不为它们生成工具输出。
用一次返回两个调用但只剩一次工具预算的案例检查。
```

“分别统计”防止将建议数量误作执行量；“不生成工具输出”保护历史事实；“两个调用但只剩一次”提供能区分错误实现的输入。**试用状态：未试用。** 使用前请阅读[《提示词示例使用说明》](../prompt-examples.md)。
