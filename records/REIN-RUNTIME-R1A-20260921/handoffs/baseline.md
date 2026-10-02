# R0 baseline and R1a implementation handoff

日期：2026-09-21  
范围：Rein R1a 原生单节点闭环，基于现有旧 `rust/`/`ts/` 基准；本文件不代表新产品代码已经实现。

## 当前边界

现有仓库只有教学包 `rust/` 和 `ts/` 主线。`core/`、`agentmux/`、`runtime/`、`TaskApplication`、持久任务库和 Rust → schemars → JSON Schema → TypeScript 生成链路尚不存在。旧实现的 [`rust/src/rein/loop.rs`](../../../rust/src/rein/loop.rs) 是 async loop：它在同一函数中 await 模型和工具，事件保存在内存 `LoopResult.events`，因此只能作为兼容行为基准，不能作为 R1a 的纯步进实现。

本轮所有 Cargo 构建均使用独立目标目录 `/tmp/rein-r0-target-20260921`。未运行真实模型、未读取密钥、未提交或发布。

## R1a 最小可落地纵切

首个实际闭环只支持一个本地原生 Harness session、一个 Run/Attempt、一个已注册的 ModelAdapter、一个只读 ToolExecutor 和一个固定 Verifier。它必须能在没有外部 Agent CLI 的环境中完成：

```text
Start observation
  -> CallModel intent
  -> ModelTurn observation (zero or more tool calls)
  -> ExecuteTool intent (one call at a time)
  -> ToolResult observation
  -> CallModel intent
  -> ModelTurn(final) observation
  -> Finish intent
  -> verifier result
  -> Acceptance only when verifier passed
```

建议的首个任务使用现有只读工具和离线 FakeModel：模型先调用 `search_files`，再调用 `read_file`，最后给出答案。该任务能复用旧 loop fixture，又能观察纯步进机是否把每个模型/工具边界提交为独立 revision。模型输入、模型输出、工具参数和工具结果必须先写入 artifact，再在 session 中保存引用、摘要、游标和控制元数据；消息正文不复制进 session 行。

R1a 的产品切片应包含以下真实组件，而不是只建空目录：

* `core/harness/agent_loop.rs`：纯 `HarnessStep::advance(state, observation)`；只计算 `Rejected | Duplicate | SessionTransition`，不 await、不读时钟、不读存储、不执行文件或网络 I/O。
* `core/domain`/`core/events`：`HarnessSession`、`Run`、`Attempt`、`Observation`、`NextIntent`、`SessionTransition` 及领域事件。效果 ID 应由 session、expected revision、调用身份和 intent 类型稳定生成。
* `core/application`：读取快照，检查 session/Run/Attempt revision、权限、预算和 artifact 引用，调用纯步进机，然后提交一次事务；提交冲突必须重读重算，不能执行未提交意图。
* `runtime/store`：R1a 可先提供单进程 SQLite 实现，但接口应明确 `load_snapshot`、`commit_transition`、`record_observation_if_new`、`pending_effects` 和 `mark_effect_outcome`。同一事务写 session patch、observation 去重、事件和待处理 effect。
* `runtime/driver/native.rs`：只消费已提交的 `CallModel`/`ExecuteTool`/`Verify` intent，await 适配器，把结果包装为 observation 再送回 application；driver 不自行重试、验收或换账号。
* `runtime/artifact`：以内容摘要保存消息和工具结果；缺失或写入失败时禁止提交悬空引用。artifact 已写入但事务失败可保留为未引用对象。
* `runtime/verify`：固定 VerificationPlan，执行一次 verifier 并保存原始回执；`failed` 保持失败，无法执行为 `undetermined`，二者都不能写 Acceptance。

R1a 暂不实现 Coordinator、真实外部 Agent、DAG、Profile 轮换和多客户端控制协议；这些保持未实现状态，不得用 FakeAgent 结果宣称 R1b 完成。

## 纯步进与 I/O driver 的最小接口

下列是建议冻结为内部 Rust API 的最小形状，字段可按正式 schema 命名调整，但语义应保持：

```text
HarnessStep::advance(
    session: &HarnessSessionState,
    observation: &Observation,
) -> StepResult

StepResult::Transition {
    expected_revision: u64,
    next_revision: u64,
    accepted_observation_id: ObservationId,
    session_patch: SessionPatch,
    next_intent: NextIntent,
    events: Vec<DomainEvent>,
}
StepResult::Duplicate { revision: u64, observation_id: ObservationId }
StepResult::Rejected { code: RejectCode, expected_revision: u64 }
```

`Observation` 至少包含 `observation_id`、`session_id`、`attempt_id`、`observed_at`（由 runtime 注入）、kind、effect/call 关联和事实引用。`Start` 只能接受一次；相同 `observation_id` 或已记录的 effect result 走 Duplicate，不推进 revision，不重新产生 effect。模型/工具结果必须携带 effect ID；迟到或未知 effect 不能直接转成功。

`NextIntent` 首轮只需 `CallModel { effect_id, model_binding, input_refs }`、`ExecuteTool { effect_id, call_id, tool, args_ref, approval_ref }`、`Verify { effect_id, plan_ref, artifact_refs }` 和 `Finish`。`Finish` 只表示规划结束，不等于验证通过或任务验收。

driver 循环应严格执行：读取待处理 effect → 只执行一个已提交 effect → 将返回值封装为 observation/artifact → 调用 application → 原子提交。若提交前失败，不派发下一 effect；若派发后进程崩溃，重启先以 effect ID/外部事实核对，结果未知时写 `outcome_unknown`，隔离并等待核对，禁止自动重发。

## 必须先冻结的 R0 基准

实现前先保存以下正反例输入和预期：

1. 正常 model → tool → model：revision 依次递增；同一观察重放不执行第二次工具。
2. 工具预算为 0 或已用尽：不产生 `ExecuteTool`，未启动调用明确记录 skipped；工作区字节为零变化。
3. 取消发生在模型等待、工具之间和 verifier 前：后续 intent 不派发，已提交效果按实际事实核对。
4. 写入/提交事务失败：不得留下 session 指向不存在 artifact 的引用，也不得派发未提交 intent。
5. 进程在 dispatch 后重启：恢复时得到 unknown/inspect 状态，不重复发出同一 effect；只有核对到结果才继续。
6. verifier 返回失败或不可执行：保存原始回执，Acceptance 不存在或保持未通过。
7. Schema 生成：同一 Rust fixture 生成 JSON Schema，再生成 TS 类型；生成物的字段、枚举、`snake_case`/`camelCase` 映射和摘要样本一致。手写第二套 DTO 应使检查失败。
8. “基准未就绪”：若 fixture、schema 或 verifier 版本未匹配当前 Spec，任务只能是 `undetermined`，不能零证据验收；若尚未进入实现阶段，受限工作区文件字节应保持不变。

这些基准先在纯步进单元测试验证决定性，再在 fake runtime/store 集成测试验证提交和重启语义，最后用旧 loop 的相同离线 fixture 做兼容对比。基准必须保存输入、预期、命令、环境、stdout/stderr、exit code 和原始日志。

## 旧 loop 兼容策略

旧入口 `run_agent_loop`、`run_agent_loop_with_adapter` 和旧 `LoopResult` 保留，先在 `rust/` 内作为薄 wrapper 使用；不要把旧 async 函数直接搬进 `core`，也不要为产品保留第二套业务循环。wrapper 的职责是把旧 `ModelAdapter`/`ToolExecutor` 适配为 runtime driver，并把新 session 的完成结果投影回旧 `LoopResult`/`LoopEvent`，保持旧测试输入、终态枚举和 wire 字段含义。

兼容桥需要单独确认：旧 `rust/Cargo.toml` 与新根 workspace 的 lockfile/feature 可以不同；两套入口分别编译和复跑基准。旧教学包继续独立存在，直到“产品原生闭环和历史入口均通过同一冻结基准”后才考虑移除 bridge。目录搬移、测试数量或新产品测试通过都不能作为退出条件。

首个实现应优先抽取旧 loop 中的消息/工具语义、预算和取消测试样例；`LoopResult.events` 的内存数组不能成为持久状态来源。旧同步维护 session、extension host 和 HTTP 监听测试分别保留为历史/环境基准，不直接等价于 R1a 持久恢复验收。

## R0 已执行基准

完整原始日志位于本目录 `evidence/`：

| 命令 | 结果 | 说明 |
| --- | --- | --- |
| `cargo fmt --manifest-path rust/Cargo.toml -- --check` | passed | 独立目标目录，exit 0 |
| `CARGO_TARGET_DIR=/tmp/rein-r0-target-20260921 cargo check --locked --manifest-path rust/Cargo.toml` | passed | exit 0 |
| `CARGO_TARGET_DIR=/tmp/rein-r0-target-20260921 cargo test ...` 全量 | failed/undetermined | 11 tests 中 6 passed、5 个 HTTP listener 测试因沙箱 `listen EPERM 127.0.0.1`；不是代码行为结论 |
| 指定旧 Rust 离线/维护/扩展测试 | partial | 相关测试通过；`prerequisites` 中 concrete reqwest listener 仍 `EPERM` |
| `npm run typecheck --workspace ts` | passed | exit 0 |
| `npm test --workspace ts` | failed/undetermined | 116 passed、7 个 listener 相关测试因 `listen EPERM 127.0.0.1` |
| TS 离线子集（chat/loop/record/hybrid-host/maintenance 等） | passed | exit 0 |
| `npm run ch05:compare-all` | passed | Rust/TS 离线回放 equal=true |
| `CARGO_TARGET_DIR=/tmp/rein-r0-target-20260921 npm run ch06:compare-all` | passed | Rust/TS 对比通过 |
| `CARGO_TARGET_DIR=/tmp/rein-r0-target-20260921 npm run ch07:compare-all` | passed | Rust/TS 对比通过 |
| `npm run hybrid:verify` | failed/undetermined | 在 TS listener 测试处停止，保留完整日志 |

旧脚本不继承专用 target 目录时曾尝试写入 `/Volumes/Data` 并被 `Operation not permitted` 拒绝；后续重跑已显式设置 `CARGO_TARGET_DIR`，原始失败也保留在对应日志中。网络/监听限制未升级权限，R0 记录为 `undetermined`，不改写测试为通过。

## R1a 进入条件与未完成项

进入实现前，主线程应冻结纯步进/观察 schema、effect idempotency key、revision 冲突策略、artifact 摘要算法、Verifier/Acceptance 状态和旧 wrapper 投影字段。当前尚未生成产品 crates、schemars schema、SQLite migration、driver 或 Apache-2.0 元数据；它们属于主线程后续实现范围。本 handoff 只提供可执行切片与基准证据。
