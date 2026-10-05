# Contracts

## 跨项目运行接口候选

[RuntimePort 0.1](runtime-port-v0.1.md) 是 Rein 与 Veriflow 之间的语言中立契约候选，规定单次执行的输入、状态、事件、取消、恢复、权限和原始产物语义。它是**待实现的设计**，不是现有 CLI、SDK 或服务接口；与教学合同分开版本化。

PR #3 的 R1a 运行时与四份 Rust 生成 schema 已合并；它们是内部运行合同，不等于 RuntimePort 已实现。候选的实现依据已更新为 PR #3 合并提交；状态映射仍须等待 RESULT-SPLIT-1 实施，不能据文档提前实现。正式 port 类型后续从一个权威定义源生成 schema 与消费者类型；Veriflow 维护工作流及整体验收，不复制 Rein 会话状态机。

## 历史教学共享合同

`ts/` 与 `rust/` 在重叠能力范围内共享的合同：消息、工具调用、工具结果的形状，以及兼容边界与版本。

从第 05 章开始，Rust core 是权威运行时；TypeScript 作为宿主或领域 plugin，通过小型语言中立的 [extension-v0.1 协议](./extension-v0.1.md)接入。provider adapter contract（第 04 章）描述模型服务商响应转换，plugin stdio 描述 core 与受控本地进程之间的调用，这两个边界不能混用。

前置版本已定义并由两条 track 读取同一份 `fixtures/cases/prerequisites.json` 验收数据；它只提供模型 turn、工具调用和只读工具路由，不包含 Loop 控制。

第05章循环事件也属于共享合同：请求事件携带完整消息和工具声明，模型事件携带完整模型消息，工具事件携带完整调用参数和结果；事件类型使用 snake_case，字段使用 camelCase wire。变化记录在 [MIGRATIONS.md](../MIGRATIONS.md)。
# ch05 前置合同

TypeScript 的 `ts/src/rein/contracts.ts` 与 Rust 的 `rust/src/rein/mod.rs` 共享四个可序列化概念：`Message` 表示对话消息，`ToolCall` 表示模型请求的工具动作，`ToolResult` 表示带 `ok` 与结构化错误的工具结果，`Turn` 把一轮消息、调用和结果成组保存。wire JSON 使用 camelCase（如 `toolCallId`、`toolCalls`、`toolResults`、`inputSchema`）；Rust 内部字段仍可使用 snake_case 并由 serde 映射。可选字段在 Rust 为 `None` 时省略，输入仍兼容缺失或 `null`，与 TS optional 对齐；`canonical_sample` 是两端共享的 canonical JSON 样例。`read_file` 与 `search_files` 只接受 workspace 内路径或关键词；工具错误码包括 `path_escape`、`path_invalid`、`read_failed`、`search_failed`、`workspace_invalid`、`arguments_invalid` 和 `unknown_tool`。TS 以 `ToolResult` 返回，Rust 也以 `ToolResult` 返回；解析与传输层错误则由 TS 抛出 `Error`、Rust 返回 `Result`。这不是对所有底层路径错误细节的隐藏承诺。

OpenAI 适配器使用现有配置和 transport，单次非流式调用，不自动重试；它序列化完整 messages/tools，并保留多个 `tool_calls` 及其 JSON arguments。Anthropic 前置适配器转换 system、text、tool_use、tool_result；回放输入是 Anthropic wire JSON，不是 `ModelTurn` 对象，按输入顺序消费，用尽后 TS 抛出 `Error('replay exhausted')`，Rust 返回 `code='replay_exhausted'`。Rust 的具体 HTTP 入口是 `rein::openai_complete`，通过 `rein::OpenAiHttp`（生产实现为 `ReqwestHttp`）发送请求；typed loop 还通过 `rein::ModelAdapter` 接收确定性离线实现，旧 `run_agent_loop` 保留为 OpenAI 包装。TS 入口是 `createOpenAIAdapter`。例如模型返回 `{"tool_calls":[{"id":"call-1","name":"read_file","arguments":{"path":"README.md"}}]}`，工具执行后追加 `{"role":"tool","toolCallId":"call-1","content":"..."}`，下一轮仍携带同一 id。第 06 章 plugin 另有 `ready`、`invoke`、`cancel` 和 `terminal` 消息，core 校验 request/task/call/rule 绑定和 evidence；Node 进程受协议约束但仍是受信任进程，不等于沙箱。
